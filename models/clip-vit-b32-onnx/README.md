---
language: en
library_name: onnxruntime
tags:
- clip
- vision
- zero-shot-classification
- image-text-similarity
- onnx
- vit-b32
pipeline_tag: zero-shot-image-classification
widget:
- text: a cat
  example_image: >-
    https://huggingface.co/datasets/huggingface/documentation-images/resolve/main/cat.png
- text: a dog
  example_image: >-
    https://huggingface.co/datasets/huggingface/documentation-images/resolve/main/dog.png
base_model:
- openai/clip-vit-base-patch32
---

# **CLIP ViT-B/32 (ONNX)**

This repository contains the **ONNX-exported version of OpenAI’s CLIP model (ViT-B/32)**, optimized for inference using [ONNX Runtime](https://onnxruntime.ai/). It supports **fast image-text similarity and zero-shot classification** without requiring PyTorch or TensorFlow.

---

## **Model Details**

* **Base Model:** [openai/clip-vit-base-patch32](https://huggingface.co/openai/clip-vit-base-patch32)
* **Export Format:** ONNX
* **Architecture:** Vision Transformer (ViT-B/32)
* **File Size:** ~600 MB (FP32 version)
* **Use Case:** Zero-shot classification, image-text similarity, and retrieval.

---

## **Quantized Models**

In addition to the standard `model.onnx` (FP32), this repo provides multiple **quantized variants** to reduce memory usage and improve inference speed:

| File                   | Precision            | Approx. Size |
| ---------------------- | -------------------- | ------------ |
| `model_fp16.onnx`      | FP16                 | ~303 MB      |
| `model_quantized.onnx` | INT8/Hybrid          | ~153 MB      |
| `model_q4.onnx`        | 4-bit                | ~189 MB      |
| `model_q4f16.onnx`     | 4-bit + FP16         | ~125 MB      |
| `model_bnb4.onnx`      | Bits-and-Bytes 4-bit | ~181 MB      |
| `model_uint8.onnx`     | 8-bit                | ~152 MB      |

> **Note:** Quantized models may have slightly lower accuracy but offer better performance and smaller size. Use them with the same ONNX Runtime API.

---

## **How to Use**

### **1. Install Dependencies**

```bash
pip install onnxruntime transformers huggingface_hub pillow numpy
````

---

### **2. Load the Model and Processor**

```python
from huggingface_hub import hf_hub_download
import onnxruntime as ort
from transformers import CLIPProcessor

# Download ONNX model from Hugging Face Hub
repo_id = "sayantan47/clip-vit-b32-onnx"
onnx_model_path = hf_hub_download(repo_id=repo_id, filename="onnx/model.onnx")

# Load ONNX Runtime session
session = ort.InferenceSession(onnx_model_path, providers=["CPUExecutionProvider"])

# Load processor
processor = CLIPProcessor.from_pretrained(repo_id)

# Example input
image = Image.open("example.jpg")
texts = ["a dog", "a cat"]

# Preprocess
inputs = processor(text=texts, images=image, return_tensors="np", padding=True)
inputs = {k: (v.astype(np.int64) if v.dtype == np.int32 else v) for k, v in inputs.items()}

# Run inference
outputs = session.run(None, inputs)
logits_per_image = outputs[0]
probs = np.exp(logits_per_image) / np.exp(logits_per_image).sum(-1, keepdims=True)
print("Probabilities:", probs)
```

---

## **Applications**

* **Zero-Shot Classification:** Classify images by comparing them to textual descriptions.
* **Image Similarity:** Compare embeddings between two images or between images and text.
* **Search Engines:** Use as the backbone for image-text retrieval systems.

---

## **ONNX Runtime Performance**

* **CPU-only:** Works out of the box with `onnxruntime` on CPUs.
* **GPU:** To use CUDA, install `onnxruntime-gpu` and ensure you have **CUDA 12 and cuDNN 9** installed.

  ```bash
  pip install onnxruntime-gpu
  ```

---

## **Export Command Used**

The model was exported using [Hugging Face Optimum](https://huggingface.co/docs/optimum/index) with:

```bash
python -m optimum.exporters.onnx --model=openai/clip-vit-base-patch32 onnx_model/
```
