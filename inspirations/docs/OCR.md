# GLM-OCR Integration

Flow now supports OCR (Optical Character Recognition) using the GLM-OCR vision model.

## Features

- Extract text from images (PNG, JPG, etc.)
- Custom prompts for specific extraction tasks
- Runs completely offline
- Fast inference with Q4_K_M quantization

## Installation

1. Download the GLM-OCR model:
```bash
python scripts/download_glm_ocr_simple.py
```

2. Install llama-cpp-python with vision support:
```bash
pip install llama-cpp-python
```

## Usage

### Basic OCR
Extract all text from an image:
```bash
cargo run --release -- --ocr document.png
```

### Custom Prompt
Use a custom prompt for specific extraction:
```bash
cargo run --release -- --ocr receipt.jpg "Extract the total amount and date"
cargo run --release -- --ocr form.png "Extract all field names and values"
cargo run --release -- --ocr screenshot.png "Summarize the main content"
```

## Examples

### Extract text from a document
```bash
flow --ocr invoice.pdf.png
```

### Extract specific information
```bash
flow --ocr business_card.jpg "Extract name, email, and phone number"
```

### Process screenshots
```bash
flow --ocr screenshot.png "What is this image showing?"
```

## Model Details

- **Model**: GLM-OCR Q4_K_M
- **Size**: 549 MB (model) + 484 MB (mmproj)
- **Location**: `models/ocr/`
- **License**: MIT
- **Source**: [zai-org/GLM-OCR](https://huggingface.co/zai-org/GLM-OCR)

## Technical Details

GLM-OCR uses a multimodal architecture with:
- Vision encoder (mmproj file) for image processing
- Language model for text generation
- Multi-Token Prediction (MTP) for improved accuracy

The model runs through llama-cpp-python's vision support, which handles:
- Image preprocessing
- Vision-language alignment
- Text generation from visual features

## Troubleshooting

### "llama-cpp-python not installed"
Install it with:
```bash
pip install llama-cpp-python
```

### "Model not found"
Run the download script:
```bash
python scripts/download_glm_ocr_simple.py
```

### Slow inference
The model uses GPU acceleration by default. If you don't have a GPU, it will fall back to CPU which is slower but still functional.
