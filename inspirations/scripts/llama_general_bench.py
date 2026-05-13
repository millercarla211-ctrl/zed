import argparse
import json
import os
import re
import sys
import time
from pathlib import Path


TASKS = [
    {
        "id": "normal-env-vars",
        "category": "normal",
        "max_tokens": 140,
        "prompt": (
            "In simple words, explain what environment variables are. Include one Windows "
            "PowerShell example. Keep it under 120 words."
        ),
        "checks": ["environment", "PowerShell", "$env:", "example"],
    },
    {
        "id": "normal-rewrite",
        "category": "normal",
        "max_tokens": 100,
        "prompt": (
            "Rewrite this message to sound clear and professional but still friendly: "
            "'bro this thing is slow and i dont know what wrong please fix asap'."
        ),
        "checks": ["slow", "fix", "please"],
    },
    {
        "id": "smart-debug",
        "category": "smart",
        "max_tokens": 190,
        "prompt": (
            "A React component fetches data in useEffect. The dependency array includes "
            "a function declared inside the component, and the page keeps fetching forever. "
            "Explain the likely cause and the safest fix in concise practical terms."
        ),
        "checks": ["useEffect", "dependency", "useCallback", "function", "rerender"],
    },
    {
        "id": "smart-tradeoff",
        "category": "smart",
        "max_tokens": 180,
        "prompt": (
            "You are choosing between a 4B model at 7 tokens/sec and a 9B model at 2 tokens/sec "
            "for local UI coding help. Give the decision rule for when to use each one."
        ),
        "checks": ["4B", "9B", "speed", "quality", "coding"],
    },
    {
        "id": "coding-next-route",
        "category": "coding",
        "max_tokens": 260,
        "prompt": (
            "Write code only for a Next.js App Router POST route handler at app/api/contact/route.ts. "
            "Validate name, email, and message without third-party packages and return NextResponse JSON."
        ),
        "checks": ["NextResponse", "POST", "request.json", "email", "message", "status"],
    },
    {
        "id": "coding-shadcn-card",
        "category": "coding",
        "max_tokens": 300,
        "prompt": (
            "Write code only for a React TypeScript shadcn/ui component named UsageCard. "
            "Use Card, CardHeader, CardContent, Badge, Button, and Tailwind classes. "
            "Props: title, used, limit, planName."
        ),
        "checks": ["Card", "CardHeader", "CardContent", "Badge", "Button", "className", "props"],
    },
]


SYSTEM_PROMPT = (
    "You are a concise local assistant for Windows, Next.js, React, Tailwind CSS, and shadcn/ui. "
    "Answer directly. Do not reveal hidden reasoning or planning. For code tasks, output code only."
)


def strip_thinking(text: str) -> str:
    text = re.sub(r"<think>.*?</think>", "", text, flags=re.IGNORECASE | re.DOTALL)
    text = re.sub(r"^\s*.*?</think>", "", text, flags=re.IGNORECASE | re.DOTALL)
    text = re.sub(r"<think>.*$", "", text, flags=re.IGNORECASE | re.DOTALL)
    return text.strip()


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


def score_answer(task: dict, content: str) -> dict:
    lowered = content.lower()
    checks = task["checks"]
    hits = sum(1 for item in checks if item.lower() in lowered)
    no_reasoning = "<think" not in lowered and "thinking process" not in lowered
    no_fence = "```" not in content
    useful_length = 30 <= len(content) <= 5000
    code_shape = task["category"] != "coding" or any(
        token in content for token in ("export ", "function ", "const ", "async function", "return ")
    )
    score = 20
    score += int((hits / max(1, len(checks))) * 45)
    score += 15 if code_shape else 0
    score += 10 if no_reasoning else 0
    score += 5 if no_fence else 0
    score += 5 if useful_length else 0
    return {
        "score": min(score, 100),
        "check_hits": hits,
        "check_total": len(checks),
        "no_reasoning": no_reasoning,
        "no_markdown_fence": no_fence,
        "useful_length": useful_length,
        "code_shape": code_shape,
    }


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
    parser.add_argument("--temperature", type=float, default=0.2)
    parser.add_argument("--top-p", type=float, default=0.9)
    parser.add_argument("--repeat-penalty", type=float, default=1.08)
    parser.add_argument("--qwen-no-think", action="store_true")
    args = parser.parse_args()

    if not args.model.exists():
        print(f"ERROR: missing model: {args.model}", file=sys.stderr)
        return 4

    args.out_dir.mkdir(parents=True, exist_ok=True)
    load_started = time.perf_counter()
    llm = load_llama(args)
    load_seconds = time.perf_counter() - load_started

    results = []
    for task in TASKS:
        user_prompt = task["prompt"]
        if args.qwen_no_think:
            user_prompt = f"/no_think\n{user_prompt}\nNo planning. No explanation."

        started = time.perf_counter()
        response = llm.create_chat_completion(
            messages=chat_messages(args, user_prompt),
            max_tokens=task["max_tokens"],
            temperature=args.temperature,
            top_p=args.top_p,
            top_k=40,
            min_p=0.05,
            repeat_penalty=args.repeat_penalty,
        )
        elapsed = time.perf_counter() - started
        content = strip_thinking(response["choices"][0]["message"]["content"])
        usage = response.get("usage", {})
        completion_tokens = int(usage.get("completion_tokens") or 0)
        prompt_tokens = int(usage.get("prompt_tokens") or 0)
        tok_s = completion_tokens / elapsed if elapsed > 0 and completion_tokens else 0.0
        output_path = args.out_dir / f"{task['id']}.txt"
        output_path.write_text(content, encoding="utf-8")
        quality = score_answer(task, content)
        result = {
            "id": task["id"],
            "category": task["category"],
            "seconds": round(elapsed, 3),
            "completion_tokens": completion_tokens,
            "prompt_tokens": prompt_tokens,
            "tokens_per_second": round(tok_s, 3),
            "chars": len(content),
            "output": str(output_path),
            **quality,
        }
        results.append(result)
        print(
            f"{task['id']}: category={task['category']} score={quality['score']} "
            f"tokens={completion_tokens} time={elapsed:.2f}s speed={tok_s:.2f} tok/s"
        )

    categories = sorted({task["category"] for task in TASKS})
    category_scores = {}
    for category in categories:
        rows = [row for row in results if row["category"] == category]
        tokens = sum(row["completion_tokens"] for row in rows)
        seconds = sum(row["seconds"] for row in rows)
        category_scores[category] = {
            "average_score": round(sum(row["score"] for row in rows) / len(rows), 1),
            "tokens_per_second": round(tokens / seconds, 3) if seconds and tokens else 0.0,
        }

    total_tokens = sum(row["completion_tokens"] for row in results)
    total_seconds = sum(row["seconds"] for row in results)
    summary = {
        "name": args.name,
        "model": str(args.model),
        "chat_format": args.chat_format,
        "qwen_no_think": args.qwen_no_think,
        "ctx": args.ctx,
        "threads": args.threads,
        "gpu_layers": args.gpu_layers,
        "load_seconds": round(load_seconds, 3),
        "average_score": round(sum(row["score"] for row in results) / len(results), 1),
        "aggregate_tokens_per_second": round(total_tokens / total_seconds, 3)
        if total_seconds and total_tokens
        else 0.0,
        "category_scores": category_scores,
        "total_completion_tokens": total_tokens,
        "total_generation_seconds": round(total_seconds, 3),
        "tasks": results,
    }
    (args.out_dir / "summary.json").write_text(
        json.dumps(summary, indent=2), encoding="utf-8"
    )
    print(json.dumps(summary, indent=2))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
