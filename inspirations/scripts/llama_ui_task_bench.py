import argparse
import json
import os
import re
import sys
import time
from pathlib import Path


TASKS = [
    {
        "id": "shadcn-stat-card",
        "prompt": (
            "Create one React TypeScript component named StatCard using shadcn/ui Card, "
            "Badge, and Button. Use Tailwind classes for a compact SaaS dashboard card. "
            "Props: title, value, delta, trend, actionLabel. Return code only."
        ),
        "checks": ["React", "Card", "Badge", "Button", "className", "export"],
    },
    {
        "id": "next-route-handler",
        "prompt": (
            "Write a Next.js App Router route handler at app/api/contact/route.ts. "
            "It should accept POST JSON with name, email, message, validate it without "
            "third-party packages, and return NextResponse JSON. Return code only."
        ),
        "checks": ["NextResponse", "POST", "request.json", "email", "message", "status"],
    },
    {
        "id": "tailwind-responsive-shell",
        "prompt": (
            "Create a responsive Next.js dashboard shell component using Tailwind CSS. "
            "It needs a sticky topbar, collapsible-looking sidebar area, mobile-first "
            "content spacing, and no external component imports. Return code only."
        ),
        "checks": ["className", "sticky", "md:", "aside", "main", "export"],
    },
    {
        "id": "shadcn-form",
        "prompt": (
            "Create a shadcn/ui styled React TypeScript settings form using Input, "
            "Label, Switch, Select, and Button. Do not use react-hook-form. Include "
            "accessible labels and local useState. Return code only."
        ),
        "checks": ["useState", "Input", "Label", "Switch", "Select", "Button"],
    },
    {
        "id": "ui-review",
        "prompt": (
            "Review this Tailwind snippet and return a concise improved JSX version only: "
            "<div className='p-12 bg-blue-500 rounded-3xl'><h1 className='text-6xl'>Settings</h1>"
            "<p>Change your account settings here</p><button>Save</button></div>. "
            "Make it calmer, shadcn-like, responsive, and production UI friendly."
        ),
        "checks": ["className", "Settings", "button", "rounded", "text-", "p-"],
    },
]

SYSTEM_PROMPT = (
    "You are a senior Next.js, React, Tailwind CSS, and shadcn/ui coding assistant. "
    "Return practical production code. Do not reveal reasoning. Do not wrap output in markdown fences."
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


def score_output(content: str, checks: list[str]) -> dict:
    lowered = content.lower()
    check_hits = sum(1 for item in checks if item.lower() in lowered)
    has_code_shape = any(token in content for token in ("export ", "function ", "const ", "return ("))
    no_reasoning = "<think" not in lowered and "reasoning" not in lowered
    no_markdown_fence = "```" not in content
    useful_length = 180 <= len(content) <= 6000
    score = 20
    score += int((check_hits / max(1, len(checks))) * 45)
    score += 15 if has_code_shape else 0
    score += 10 if no_reasoning else 0
    score += 5 if no_markdown_fence else 0
    score += 5 if useful_length else 0
    return {
        "score": min(score, 100),
        "check_hits": check_hits,
        "check_total": len(checks),
        "has_code_shape": has_code_shape,
        "no_reasoning": no_reasoning,
        "no_markdown_fence": no_markdown_fence,
        "useful_length": useful_length,
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
    parser.add_argument("--ctx", type=int, default=int(os.environ.get("FLOW_BENCH_CTX", "4096")))
    parser.add_argument("--batch", type=int, default=512)
    parser.add_argument("--ubatch", type=int, default=256)
    parser.add_argument("--threads", type=int, default=int(os.environ.get("FLOW_BENCH_THREADS", "6")))
    parser.add_argument("--gpu-layers", type=int, default=0)
    parser.add_argument("--max-tokens", type=int, default=420)
    parser.add_argument("--temperature", type=float, default=0.25)
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
            user_prompt = f"/no_think\n{user_prompt}\nNo planning. No explanation. Code only."
        started = time.perf_counter()
        response = llm.create_chat_completion(
            messages=[
                {"role": "system", "content": SYSTEM_PROMPT},
                {"role": "user", "content": user_prompt},
            ],
            max_tokens=args.max_tokens,
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
        score = score_output(content, task["checks"])
        results.append(
            {
                "id": task["id"],
                "seconds": round(elapsed, 3),
                "completion_tokens": completion_tokens,
                "prompt_tokens": prompt_tokens,
                "tokens_per_second": round(tok_s, 3),
                "chars": len(content),
                "output": str(output_path),
                **score,
            }
        )
        print(
            f"{task['id']}: score={score['score']} tokens={completion_tokens} "
            f"time={elapsed:.2f}s speed={tok_s:.2f} tok/s"
        )

    avg_score = sum(item["score"] for item in results) / len(results)
    total_tokens = sum(item["completion_tokens"] for item in results)
    total_seconds = sum(item["seconds"] for item in results)
    avg_tok_s = total_tokens / total_seconds if total_seconds > 0 and total_tokens else 0.0
    summary = {
        "name": args.name,
        "model": str(args.model),
        "chat_format": args.chat_format,
        "ctx": args.ctx,
        "threads": args.threads,
        "gpu_layers": args.gpu_layers,
        "load_seconds": round(load_seconds, 3),
        "average_score": round(avg_score, 1),
        "aggregate_tokens_per_second": round(avg_tok_s, 3),
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
