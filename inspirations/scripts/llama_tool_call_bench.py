import argparse
import json
import os
import re
import sys
import time
from pathlib import Path


TOOLS = [
    {
        "name": "answer_direct",
        "description": "Answer directly when no external action is needed.",
        "schema": {"answer": "string"},
    },
    {
        "name": "read_file",
        "description": "Read a local workspace file before answering.",
        "schema": {"path": "string"},
    },
    {
        "name": "run_tests",
        "description": "Run a local test or verification command.",
        "schema": {"command": "string"},
    },
    {
        "name": "search_web",
        "description": "Look up current external information.",
        "schema": {"query": "string"},
    },
    {
        "name": "create_ui_component",
        "description": "Create or edit a UI component in the codebase.",
        "schema": {"component": "string", "stack": "string"},
    },
]

TASKS = [
    {
        "id": "direct-answer",
        "expected_tool": "answer_direct",
        "prompt": "What is an environment variable? Answer simply.",
        "arg_keywords": ["environment"],
    },
    {
        "id": "read-file",
        "expected_tool": "read_file",
        "prompt": "Open README.md and summarize the install commands.",
        "arg_keywords": ["README"],
    },
    {
        "id": "run-tests",
        "expected_tool": "run_tests",
        "prompt": "Check whether the Rust LLM module tests pass.",
        "arg_keywords": ["cargo", "test"],
    },
    {
        "id": "search-current",
        "expected_tool": "search_web",
        "prompt": "Find the latest official Next.js App Router route handler docs.",
        "arg_keywords": ["Next.js", "route"],
    },
    {
        "id": "ui-edit",
        "expected_tool": "create_ui_component",
        "prompt": "Make a shadcn/ui UsageCard component for the dashboard.",
        "arg_keywords": ["UsageCard", "shadcn"],
    },
]

SYSTEM_PROMPT = (
    "You are a tool-routing model for a local chat UI. Choose exactly one tool. "
    "Return strict JSON only, with this shape: "
    "{\"tool\":\"tool_name\",\"arguments\":{...}}. "
    "No markdown. No explanation. No hidden reasoning."
)


def strip_artifacts(text: str) -> str:
    text = re.sub(r"<think>.*?</think>", "", text, flags=re.IGNORECASE | re.DOTALL)
    text = re.sub(r"^\s*.*?</think>", "", text, flags=re.IGNORECASE | re.DOTALL)
    text = re.sub(r"<think>.*$", "", text, flags=re.IGNORECASE | re.DOTALL)
    return (
        text.replace("<end_of_turn>", "")
        .replace("<|im_end|>", "")
        .replace("<|endoftext|>", "")
        .strip()
    )


def extract_json(text: str):
    try:
        return json.loads(text)
    except json.JSONDecodeError:
        match = re.search(r"\{.*\}", text, flags=re.DOTALL)
        if not match:
            raise
        return json.loads(match.group(0))


def load_llama(args):
    try:
        from llama_cpp import Llama
    except ImportError:
        print(
            "ERROR: llama-cpp-python is not installed. Install with: python -m pip install llama-cpp-python",
            file=sys.stderr,
        )
        raise SystemExit(2)

    kwargs = {
        "model_path": str(args.model),
        "n_ctx": args.ctx,
        "n_batch": args.batch,
        "n_ubatch": args.ubatch,
        "n_threads": args.threads,
        "n_threads_batch": args.threads,
        "n_gpu_layers": args.gpu_layers,
        "verbose": False,
    }
    if args.chat_format != "auto":
        kwargs["chat_format"] = args.chat_format
    return Llama(**kwargs)


def chat_messages(args, user_prompt: str):
    if args.chat_format == "gemma":
        return [{"role": "user", "content": f"{SYSTEM_PROMPT}\n\n{user_prompt}"}]

    return [
        {"role": "system", "content": SYSTEM_PROMPT},
        {"role": "user", "content": user_prompt},
    ]


def main() -> int:
    if hasattr(sys.stdout, "reconfigure"):
        sys.stdout.reconfigure(encoding="utf-8", errors="replace")
    if hasattr(sys.stderr, "reconfigure"):
        sys.stderr.reconfigure(encoding="utf-8", errors="replace")

    parser = argparse.ArgumentParser()
    parser.add_argument("--model", required=True, type=Path)
    parser.add_argument("--name", required=True)
    parser.add_argument("--out-dir", required=True, type=Path)
    parser.add_argument("--chat-format", default="auto")
    parser.add_argument("--ctx", type=int, default=int(os.environ.get("FLOW_BENCH_CTX", "3072")))
    parser.add_argument("--batch", type=int, default=512)
    parser.add_argument("--ubatch", type=int, default=256)
    parser.add_argument("--threads", type=int, default=int(os.environ.get("FLOW_BENCH_THREADS", "6")))
    parser.add_argument("--gpu-layers", type=int, default=0)
    parser.add_argument("--temperature", type=float, default=0.0)
    parser.add_argument("--top-p", type=float, default=0.9)
    args = parser.parse_args()

    if not args.model.exists():
        print(f"ERROR: missing model: {args.model}", file=sys.stderr)
        return 4

    args.out_dir.mkdir(parents=True, exist_ok=True)
    load_started = time.perf_counter()
    llm = load_llama(args)
    load_seconds = time.perf_counter() - load_started

    tools_json = json.dumps(TOOLS, separators=(",", ":"))
    results = []
    for task in TASKS:
        prompt = (
            f"Available tools: {tools_json}\n"
            f"User request: {task['prompt']}\n"
            "Choose the tool and arguments."
        )
        started = time.perf_counter()
        response = llm.create_chat_completion(
            messages=chat_messages(args, prompt),
            max_tokens=160,
            temperature=args.temperature,
            top_p=args.top_p,
            top_k=40,
            min_p=0.05,
            repeat_penalty=1.05,
        )
        elapsed = time.perf_counter() - started
        raw = response["choices"][0]["message"]["content"]
        content = strip_artifacts(raw)
        output_path = args.out_dir / f"{task['id']}.txt"
        output_path.write_text(content, encoding="utf-8")
        usage = response.get("usage", {})
        completion_tokens = int(usage.get("completion_tokens") or 0)

        parsed = None
        parse_error = None
        try:
            parsed = extract_json(content)
        except Exception as exc:  # noqa: BLE001 - report exact model-output parse issue
            parse_error = str(exc)

        tool = parsed.get("tool") if isinstance(parsed, dict) else None
        arguments = parsed.get("arguments") if isinstance(parsed, dict) else None
        arg_blob = json.dumps(arguments, ensure_ascii=False) if arguments is not None else ""
        keyword_hits = sum(1 for keyword in task["arg_keywords"] if keyword.lower() in arg_blob.lower())
        score = 0
        score += 45 if parsed is not None else 0
        score += 35 if tool == task["expected_tool"] else 0
        score += 15 if keyword_hits > 0 else 0
        score += 5 if content.strip().startswith("{") and content.strip().endswith("}") else 0
        tok_s = completion_tokens / elapsed if elapsed > 0 and completion_tokens else 0.0
        row = {
            "id": task["id"],
            "expected_tool": task["expected_tool"],
            "tool": tool,
            "score": score,
            "seconds": round(elapsed, 3),
            "completion_tokens": completion_tokens,
            "tokens_per_second": round(tok_s, 3),
            "parse_error": parse_error,
            "output": str(output_path),
        }
        results.append(row)
        print(
            f"{task['id']}: score={score} expected={task['expected_tool']} got={tool} "
            f"tokens={completion_tokens} speed={tok_s:.2f} tok/s"
        )

    total_tokens = sum(row["completion_tokens"] for row in results)
    total_seconds = sum(row["seconds"] for row in results)
    summary = {
        "name": args.name,
        "model": str(args.model),
        "chat_format": args.chat_format,
        "ctx": args.ctx,
        "threads": args.threads,
        "load_seconds": round(load_seconds, 3),
        "average_score": round(sum(row["score"] for row in results) / len(results), 1),
        "aggregate_tokens_per_second": round(total_tokens / total_seconds, 3)
        if total_seconds and total_tokens
        else 0.0,
        "tasks": results,
    }
    (args.out_dir / "summary.json").write_text(
        json.dumps(summary, indent=2), encoding="utf-8"
    )
    print(json.dumps(summary, indent=2))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
