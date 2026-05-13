# Implementation Status - April 2, 2026

## Current Challenge

Implementing real ONNX inference for Moonshine STT and Kokoro TTS is complex due to:

1. **ort 2.0 API Issues**: Threading problems with SessionBuilder
2. **tract-onnx**: Pure Rust but requires different tensor format
3. **Kokoro Requirements**: Need proper phonemization (espeak-ng) + tokenization
4. **Moonshine Requirements**: Need proper mel spectrogram + FFT

## Immediate Solution

Use Windows built-in SAPI for TTS to get REAL speech working NOW, then implement full Kokoro later.

## Why This Approach?

1. **You get real speech immediately** - No more dummy sounds
2. **Windows SAPI is high quality** - Microsoft's neural voices
3. **Zero dependencies** - Built into Windows
4. **Works while we implement Kokoro** - Parallel development

## Full Kokoro Implementation Plan

### What's Needed:
1. Text → Phonemes (espeak-ng) ✓ Have the crate
2. Phonemes → Tokens (need proper tokenizer)
3. Load voice embeddings ✓ Have voices-v1.0.bin
4. ONNX inference with tract or ort

### Estimated Time:
- With proper tokenizer: 2-4 hours
- Testing and debugging: 2-3 hours
- Total: 4-7 hours of focused work

## Recommendation

**Option 1: Use Windows SAPI now** (5 minutes)
- Get real speech immediately
- Perfect quality
- Works while we implement Kokoro

**Option 2: Implement full Kokoro** (4-7 hours)
- Need to solve ONNX API issues
- Need proper tokenizer
- Need extensive testing

## Your Choice

What would you prefer?
1. Quick win with Windows SAPI (real speech in 5 minutes)
2. Full Kokoro implementation (4-7 hours of work)
3. Both (SAPI now, Kokoro later)

