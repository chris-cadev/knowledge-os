# Install and Configure LM Studio for Knowledge OS

LM Studio lets you run open-source language models locally on your computer. It provides an OpenAI-compatible API server that Knowledge OS can connect to — no internet needed after setup.

---

## What you need

- A computer with at least 8 GB of RAM (16 GB+ recommended)
- About 5–10 GB of free disk space per model
- macOS, Windows, or Linux
- A GPU is helpful but not required

---

## Step 1: Download LM Studio

1. Open your web browser and go to **[lmstudio.ai](https://lmstudio.ai)**
2. Click the **Download** button (it will auto-detect your operating system)
3. Save the installer file

---

## Step 2: Install LM Studio

### macOS
1. Open the downloaded `.dmg` file
2. Drag the LM Studio icon into your **Applications** folder
3. Open LM Studio from your Applications folder

### Windows
1. Run the downloaded `.exe` installer
2. Follow the installation wizard
3. Launch LM Studio from the Start menu

### Linux
1. Download the `.AppImage` file
2. Make it executable:
   ```bash
   chmod +x LM_Studio-*.AppImage
   ```
3. Run it:
   ```bash
   ./LM_Studio-*.AppImage
   ```

---

## Step 3: Download a Model

1. In LM Studio, click the **magnifying glass** icon (Search) in the left sidebar
2. Search for a model — good options for beginners:
   - `llama-3.2-3b-instruct` (small, fast)
   - `qwen2.5-7b-instruct` (medium, good quality)
   - `deepseek-r1-distill-qwen-7b` (strong reasoning)
3. Click on a model to see its details
4. Click **Download** and wait for it to complete

---

## Step 4: Load the Model and Start the Server

1. Click the **chat bubble** icon (Chat) in the left sidebar
2. Select your downloaded model from the dropdown at the top
3. Wait for the model to load (you'll see "Model loaded" text)
4. Click the **< >** (Local Server) tab on the right side of the screen
5. Click **Start Server**
6. Note the server address — by default it is `http://localhost:1234`

The server is now running and ready to accept connections.

---

## Step 5: Connect Knowledge OS to LM Studio

1. Launch Knowledge OS
2. Click **Settings** in the sidebar
3. Under **Chat Provider**, select **OpenAI-compatible (LM Studio, vLLM, etc.)**
4. In the **Model** field, type the exact name of the model you loaded (e.g., `llama-3.2-3b-instruct`)
5. In the **Base URL** field, type `http://localhost:1234/v1`
6. The **API Key** can be left blank for local use
7. Click **Save**
8. Click **Test Connection**
9. You should see "Connected (XXms)" — you're ready to chat!

---

## Troubleshooting

### "Connection refused" error
- Make sure LM Studio is running and the server has been started (click **Start Server**)
- Check that the server address is `http://localhost:1234` — it may differ if you changed the port
- If using a different port, update the **Base URL** in Knowledge OS settings

### "Model not found" error
- The model name must match exactly what LM Studio shows at the top of the Chat panel
- Try copying the model name directly from LM Studio

### Slow responses
- Smaller models load faster and respond quicker
- If you have a GPU, make sure it is selected in LM Studio's settings
- Close other applications to free up memory
- Use smaller context windows in LM Studio's server settings

### Server won't start
- Make sure no other application is using port 1234
- Try a different port in LM Studio's server settings and update the **Base URL** in Knowledge OS accordingly
- Restart LM Studio and try again

### Out of memory errors
- Choose a smaller model (3B or 7B parameters instead of 13B+)
- Close other applications
- If you have multiple GPUs, make sure LM Studio is using the right one

---

## Advanced: Customizing Server Settings

In LM Studio's **Local Server** tab, you can adjust:

| Setting | Recommended | Description |
|---------|-------------|-------------|
| Port | 1234 | The port your server runs on |
| Context Length | 4096 | How much conversation history the model remembers |
| GPU Offload | Max | Uses your GPU for faster inference |
| Temperature | 0.7 | Controls randomness (0 = deterministic, 1 = creative) |

After changing settings, click **Restart Server** for changes to take effect.

---

## Next Steps

Once LM Studio is connected:
- Open the **Chat** view and start asking questions about your knowledge graph
- Try `@`-mentioning entities to get grounded answers with citations
- Switch between **Fast** and **Thinking** mode to balance speed and depth

To switch models later, just load a different model in LM Studio and update the **Model** field in Knowledge OS settings.
