# Install and Configure Ollama for Knowledge OS

Ollama lets you run large language models locally on your computer — no internet connection needed, no API keys, no monthly fees.

---

## What you need

- A computer with at least 8 GB of RAM (16 GB+ recommended for larger models)
- About 5 GB of free disk space for a small model (llama3.2)
- macOS, Windows, or Linux

---

## Step 1: Download Ollama

1. Open your web browser and go to **[ollama.com](https://ollama.com)**
2. Click the **Download** button
3. Choose your operating system:
   - **macOS:** Download the `.dmg` file
   - **Windows:** Download the `.exe` installer
   - **Linux:** Copy the curl command shown on the page and run it in your terminal

---

## Step 2: Install Ollama

### macOS
1. Open the downloaded `.dmg` file
2. Drag the Ollama icon into your **Applications** folder
3. Open Ollama from your Applications folder
4. You will see a llama icon in your menu bar — Ollama is running

### Windows
1. Run the downloaded `.exe` installer
2. Follow the installation wizard
3. Ollama will start automatically and run in your system tray

### Linux
1. Open a terminal
2. Run the curl command from the website:
   ```bash
   curl -fsSL https://ollama.com/install.sh | sh
   ```
3. Wait for the installation to complete

---

## Step 3: Download a Model

Open a terminal (or command prompt on Windows) and run:

```bash
ollama pull llama3.2
```

This downloads a small, fast model (about 2 GB). For better results, you can also try:

```bash
ollama pull deepseek-r1:8b    # Strong reasoning — about 4.7 GB
ollama pull qwen2.5:7b        # Good all-around — about 4.4 GB
```

You only need to download a model once. After downloading, the model stays on your computer and is ready to use anytime Ollama is running.

---

## Step 4: Verify Ollama is Running

In your terminal, run:

```bash
ollama list
```

You should see the model(s) you downloaded listed. If you see an error, make sure Ollama is running (check your menu bar or system tray).

---

## Step 5: Connect Knowledge OS to Ollama

1. Launch Knowledge OS
2. Click **Settings** in the sidebar
3. Under **Chat Provider**, select **Ollama (local, free)**
4. In the **Model** field, type `llama3.2` (or whichever model you downloaded)
5. Leave the **Base URL** as `http://localhost:11434` (Ollama's default)
6. Click **Save**
7. Click **Test Connection**
8. You should see "Connected (XXms)" — you're ready to chat!

---

## Troubleshooting

### "Connection refused" error
- Make sure Ollama is running (check your menu bar or system tray)
- Wait a few seconds after starting Ollama — it takes a moment to be ready

### "Model not found" error
- Run `ollama pull llama3.2` in your terminal and wait for it to complete
- Make sure you typed the model name correctly in Knowledge OS

### Slow responses
- Smaller models (llama3.2, qwen2.5:3b) respond faster
- Larger models (deepseek-r1:8b, llama3.1:8b) give better answers but take longer
- For faster responses, switch to **Fast** mode in the chat toolbar

### Ollama won't start on macOS
- Go to **System Settings > Privacy & Security**
- Scroll down and look for a message about Ollama being blocked
- Click **Allow Anyway**

---

## Next Steps

Once Ollama is connected:
- Open the **Chat** view and start asking questions about your knowledge graph
- Try `@`-mentioning entities to get grounded answers with citations
- Switch between **Fast** and **Thinking** mode to balance speed and depth

To install other models: `ollama pull <model-name>` — browse available models at [ollama.com/library](https://ollama.com/library)
