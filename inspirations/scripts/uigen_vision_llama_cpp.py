import argparse
import base64
import contextlib
import mimetypes
import os
import sys
from pathlib import Path


def image_data_url(path: Path) -> str:
    mime = mimetypes.guess_type(path.name)[0] or "image/png"
    data = base64.b64encode(path.read_bytes()).decode("ascii")
    return f"data:{mime};base64,{data}"


def load_llm(args):
    try:
        from llama_cpp import Llama
    except ImportError:
        print(
            "ERROR: llama-cpp-python is not installed. Install with: python -m pip install llama-cpp-python",
            file=sys.stderr,
        )
        sys.exit(2)

    common = {
        "model_path": str(args.model),
        "n_ctx": args.ctx,
        "n_threads": args.threads,
        "n_gpu_layers": 0,
        "verbose": False,
    }

    handlers = vision_handler_candidates(args)
    errors = []

    # llama-cpp-python exposes local vision support through chat handlers.
    # Avoid passing clip_model_path directly to Llama: __init__ accepts **kwargs,
    # so unsupported projector args can be silently ignored by some builds.
    for label, factory in handlers:
        try:
            return Llama(
                **common,
                chat_handler=factory(str(args.mmproj)),
            )
        except Exception as exc:
            errors.append(f"{label}={exc}")

    print(
        "ERROR: llama-cpp-python does not expose a compatible local vision handler. "
        "Upgrade with: python -m pip install -U llama-cpp-python\n"
        f"DETAIL: {'; '.join(errors)}",
        file=sys.stderr,
    )
    sys.exit(3)


def vision_handler_candidates(args):
    from llama_cpp import llama_chat_format

    handlers = {
        "llava15": getattr(llama_chat_format, "Llava15ChatHandler", None),
        "llava16": getattr(llama_chat_format, "Llava16ChatHandler", None),
        "qwen25vl": getattr(llama_chat_format, "Qwen25VLChatHandler", None),
        "minicpm": getattr(llama_chat_format, "MiniCPMv26ChatHandler", None),
    }

    if args.chat_handler != "auto":
        handler = handlers.get(args.chat_handler)
        if handler is None:
            available = ", ".join(name for name, value in handlers.items() if value)
            print(
                f"ERROR: chat handler '{args.chat_handler}' is unavailable. Available: {available}",
                file=sys.stderr,
            )
            sys.exit(3)
        return [(args.chat_handler, handler)]

    model_name = args.model.name.lower()
    mmproj_name = args.mmproj.name.lower()
    if "qwen" in model_name or "qwen" in mmproj_name:
        order = ["qwen25vl", "llava16", "llava15"]
    elif "minicpm" in model_name or "minicpm" in mmproj_name:
        order = ["minicpm", "llava16", "llava15"]
    else:
        order = ["llava15", "llava16", "qwen25vl"]

    return [(name, handlers[name]) for name in order if handlers.get(name)]


@contextlib.contextmanager
def silence_native_logs():
    devnull = os.open(os.devnull, os.O_WRONLY)
    old_stdout = os.dup(1)
    old_stderr = os.dup(2)
    try:
        os.dup2(devnull, 1)
        os.dup2(devnull, 2)
        yield
    finally:
        os.dup2(old_stdout, 1)
        os.dup2(old_stderr, 2)
        os.close(old_stdout)
        os.close(old_stderr)
        os.close(devnull)


def main() -> int:
    if hasattr(sys.stdout, "reconfigure"):
        sys.stdout.reconfigure(encoding="utf-8", errors="replace")
    if hasattr(sys.stderr, "reconfigure"):
        sys.stderr.reconfigure(encoding="utf-8", errors="replace")

    parser = argparse.ArgumentParser()
    parser.add_argument("--model", required=True, type=Path)
    parser.add_argument("--mmproj", required=True, type=Path)
    parser.add_argument("--image", required=True, type=Path)
    parser.add_argument("--prompt", required=True)
    parser.add_argument("--max-tokens", type=int, default=2500)
    parser.add_argument("--ctx", type=int, default=8192)
    parser.add_argument("--threads", type=int, default=4)
    parser.add_argument(
        "--chat-handler",
        choices=("auto", "llava15", "llava16", "qwen25vl", "minicpm"),
        default="auto",
    )
    args = parser.parse_args()

    for required in (args.model, args.mmproj, args.image):
        if not required.exists():
            print(f"ERROR: missing file: {required}", file=sys.stderr)
            return 4

    messages = [
        {
            "role": "user",
            "content": [
                {"type": "text", "text": args.prompt},
                {"type": "image_url", "image_url": {"url": image_data_url(args.image)}},
            ],
        }
    ]
    with silence_native_logs():
        llm = load_llm(args)
        response = llm.create_chat_completion(
            messages=messages,
            max_tokens=args.max_tokens,
            temperature=0.25,
            top_p=0.9,
            top_k=40,
            min_p=0.05,
            repeat_penalty=1.18,
            frequency_penalty=0.25,
            presence_penalty=0.05,
        )
    print(response["choices"][0]["message"]["content"])
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
