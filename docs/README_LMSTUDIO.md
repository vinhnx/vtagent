# LMStudio Integration with VTAgent

This directory contains files to help you use VTAgent with LMStudio for local AI assistance.

## Files Included

1. **`vtagent_minimal.toml`** - Minimal configuration file for LMStudio
2. **`test_lmstudio_connection.sh`** - Script to test LMStudio connectivity
3. **`test_lmstudio_simple.sh`** - Another connectivity test script
4. **`lmstudio_example.rs`** - Example Rust code showing LMStudio integration
5. **`docs/LMSTUDIO_QUICK_START_GUIDE.md`** - Complete guide for setting up LMStudio with VTAgent

## Quick Start

1. **Install LMStudio**
   - Download from [https://lmstudio.ai/](https://lmstudio.ai/)
   - Install and launch LMStudio

2. **Load a Model**
   - In LMStudio, go to "Local Inference" tab
   - Select or download a model (Qwen3, Llama3.1, etc.)
   - Click "Start Server"

3. **Test Connection**
   ```bash
   ./test_lmstudio_connection.sh
   ```

4. **Configure VTAgent**
   - Copy `vtagent_minimal.toml` to `vtagent.toml`
   - Update the model name to match your LMStudio model

5. **Run VTAgent**
   ```bash
   cargo run
   ```

## Documentation

For detailed instructions, see:
- [`docs/LMSTUDIO_QUICK_START_GUIDE.md`](docs/LMSTUDIO_QUICK_START_GUIDE.md) - Complete setup guide
- [`docs/LMSTUDIO_SETUP_GUIDE.md`](docs/LMSTUDIO_SETUP_GUIDE.md) - Alternative setup guide

## Troubleshooting

If you encounter issues:

1. **Connection Failed**
   - Ensure LMStudio is running
   - Verify the server is started in LMStudio
   - Check that LMStudio is using port 1234

2. **Model Not Found**
   - Double-check the model name in your `vtagent.toml`
   - Ensure it exactly matches the name in LMStudio

3. **Slow Performance**
   - Try a smaller model
   - Close other resource-intensive applications

## Requirements

- LMStudio 0.3.0 or later
- Rust 1.70 or later
- curl (for testing scripts)
- jq (optional, for better JSON formatting in tests)