"""
Zenith AI - Hugging Face Spaces Demo

This is an interactive demo of Zenith AI's capabilities.
"""

import gradio as gr

def benchmark_comparison():
 """Show benchmark comparison data."""
 data = """
 | Task | Standard PyTorch | Zenith AI | Speedup |
 |------|------------------|-----------|---------|
 | ImageNet 1TB Loading | 45 min | 8 min | **5.6x** |
 | Text Tokenization (10M docs) | 12 min | 2 min | **6x** |
 | Real-time Inference | 50K events/s | 6M events/s | **120x** |
 """
 return data

def code_example(framework):
 """Return code example for selected framework."""
 if framework == "PyTorch":
 return '''import zenith.torch as zt

loader = zt.DataLoader(
 source="path/to/training_data",
 batch_size=64,
 shuffle=True,
 preprocessing_plugin="image_resize.wasm",
 num_workers=4,
 pin_memory=True
)

for epoch in range(10):
 for batch in loader:
 outputs = model(batch)
 loss = criterion(outputs, targets)
 loss.backward()
 optimizer.step()'''
 
 elif framework == "TensorFlow":
 return '''import zenith.tensorflow as ztf

dataset = ztf.ZenithDataset(
 source="path/to/training_data",
 preprocessing_plugin="image_resize.wasm"
)

dataset = dataset.batch(32).prefetch(tf.data.AUTOTUNE)

model.fit(dataset, epochs=10)'''
 
 else:
 return '''import zenith

engine = zenith.Engine(buffer_size=4096)
engine.load_plugin("preprocess.wasm")

data = engine.load("path/to/data")
processed = engine.process(data)'''

def get_install_command(extras):
 """Generate install command based on selected extras."""
 if extras == "Basic":
 return "pip install zenith-ai"
 elif extras == "PyTorch":
 return "pip install zenith-ai[torch]"
 elif extras == "TensorFlow":
 return "pip install zenith-ai[tensorflow]"
 else:
 return "pip install zenith-ai[all]"
# Create Gradio interface
with gr.Blocks(title="Zenith AI Demo", theme=gr.themes.Soft()) as demo:
 gr.Markdown("""
# Zenith AI
### High-Performance Data Infrastructure for Machine Learning
 
 > *"Stop Starving Your GPUs. Feed Them with Zenith."*
 
 Zenith AI is a Rust-powered data loading library that's **10-120x faster** than standard PyTorch/TensorFlow DataLoaders.
 """)
 
 with gr.Tab(" Benchmarks"):
 gr.Markdown(benchmark_comparison())
 gr.Markdown("""
**Why is Zenith faster?**
- **Rust Core**: No Python GIL limitations
- **Zero-Copy**: Apache Arrow memory management
- **Lock-free Buffers**: Optimized for streaming
- **WASM Plugins**: Fast preprocessing without Python
 """)
 
 with gr.Tab(" Code Examples"):
 framework = gr.Radio(
 ["PyTorch", "TensorFlow", "Basic"], 
 label="Select Framework",
 value="PyTorch"
 )
 code_output = gr.Code(language="python", label="Example Code")
 framework.change(code_example, framework, code_output)
 demo.load(lambda: code_example("PyTorch"), outputs=code_output)
 
 with gr.Tab(" Installation"):
 extras = gr.Radio(
 ["Basic", "PyTorch", "TensorFlow", "All"],
 label="Select Installation Type",
 value="Basic"
 )
 install_cmd = gr.Code(language="bash", label="Install Command")
 extras.change(get_install_command, extras, install_cmd)
 demo.load(lambda: get_install_command("Basic"), outputs=install_cmd)
 
 gr.Markdown("""
### Requirements
- Python 3.10+
- Linux or macOS (Windows coming soon)
### Links
- [GitHub Repository](https://github.com/vibeswithkk/Zenith-dataplane)
- [PyPI Package](https://pypi.org/project/zenith-ai/)
- [Documentation](https://github.com/vibeswithkk/Zenith-dataplane#documentation)
 """)
 
 gr.Markdown("""
---
**Created by Wahyu Ardiansyah** | [GitHub](https://github.com/vibeswithkk) | Apache 2.0 License
 """)

if __name__ == "__main__":
 demo.launch()
