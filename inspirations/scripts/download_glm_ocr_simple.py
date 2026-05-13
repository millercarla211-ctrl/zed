#!/usr/bin/env python3
"""Download GLM-OCR model files from HuggingFace"""

import os
from huggingface_hub import hf_hub_download

MODEL_REPO = "mradermacher/GLM-OCR-GGUF"
OUTPUT_DIR = "models/ocr"

files = [
    ("GLM-OCR.Q4_K_M.gguf", "549 MB"),
    ("GLM-OCR.mmproj-Q8_0.gguf", "484 MB"),
]

print("🤖 Downloading GLM-OCR Model")
print("=" * 50)
print(f"Repository: {MODEL_REPO}")
print(f"Output: {OUTPUT_DIR}")
print()

# Create output directory
os.makedirs(OUTPUT_DIR, exist_ok=True)

for filename, size in files:
    print(f"📥 Downloading {filename} ({size})...")
    try:
        hf_hub_download(
            repo_id=MODEL_REPO,
            filename=filename,
            local_dir=OUTPUT_DIR,
            local_dir_use_symlinks=False
        )
        print(f"✓ {filename} downloaded successfully!")
    except Exception as e:
        print(f"✗ Failed to download {filename}: {e}")
        exit(1)

print()
print("✓ All files downloaded successfully!")
print(f"Location: {os.path.abspath(OUTPUT_DIR)}")
