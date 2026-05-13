# Flow Wake-Word Training

Flow uses LiveKit-compatible wake-word models at runtime and keeps the always-on path tiny:

- Wake/VAD can stay resident.
- STT loads only after a wake command or push-to-talk action.
- Generated training audio, checkpoints, features, and ONNX artifacts stay out of git.

## Commands

Train one model per command and place the exported classifier at:

| Command | Phrase | Runtime file |
| --- | --- | --- |
| `dx` | `dx` | `models/wake_words/dx.onnx` |
| `friday` | `friday` | `models/wake_words/friday.onnx` |
| `hello` | `hello` | `models/wake_words/hello.onnx` |
| `aladdin` | `aladdin` | `models/wake_words/aladdin.onnx` |
| `arise` | `arise` | `models/wake_words/arise.onnx` |

Each template lives under `configs/wakewords/` and uses:

- `model_type: conv_attention`
- `model_size: small`
- `n_samples: 10000`
- `steps: 50000`
- `target_fp_per_hour: 0.2`

## Training Handoff

Run the training pipeline in Colab, Linux, or WSL. This should not run as part of Flow's resident runtime.

For example, to train `dx`:

```powershell
powershell -ExecutionPolicy Bypass -File scripts/wakeword-training-handoff.ps1 -Command dx
```

The script prints the exact command handoff. The underlying LiveKit flow is:

```bash
cd vendor/livekit-wakeword
uv sync --all-extras
uv run livekit-wakeword setup --config ../../configs/wakewords/dx.yaml
uv run livekit-wakeword run ../../configs/wakewords/dx.yaml
```

After export, copy the generated ONNX classifier to:

```text
models/wake_words/dx.onnx
```

Repeat for `friday`, `hello`, `aladdin`, and `arise`.

## Runtime Behavior

Flow discovers only ONNX files present in `models/wake_words`. If a model is missing, text alias matching still recognizes the canonical command phrases for non-audio tests and host integrations.

The runtime threshold is `68%` for these LiveKit conv-attention classifiers, with a `1500ms` debounce.

## Readiness Check

Use the voice status script to see wake, STT, and runtime readiness in one place:

```powershell
powershell -ExecutionPolicy Bypass -File scripts/flow-voice-status.ps1
```
