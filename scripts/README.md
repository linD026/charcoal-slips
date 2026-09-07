# Scripts

## Setup the remote serever with Ollama 

**1. Run remote server alongside local server (avoids port collisions):**
If your local Ollama is using `11434`, map the remote server to `11435`.

```bash
./setup_server.sh remote -r machine01 -lp 11435 -rp 11434

```

*(Now you can set `ccslips` to use `http://localhost:11435/api/generate` for the remote AI, and leave standard tools looking at `11434` for the local AI).*

**2. Tear down the specific tunnel:**

```bash
./setup_server.sh local -lp 11435

```

**3. Standard override (if local Ollama is offline):**

```bash
./setup_server.sh remote -r machine02

```

## Test the remote server

1. **Basic Connection Test:**
Tests the default port (`11434`) to see if the server responds and lists its available models.
```bash
./test_remote.sh

```

2. **Test a Custom Port:**
If you tunneled the remote server to `11435` in the previous step, test that specific port:
```bash
./test_remote.sh -p 11435

```

3. **Test Actual AI Generation:**
Add the `-g` flag to actually send a test prompt to the first model it finds on the remote server to prove the AI can process requests.
```bash
./test_remote.sh -p 11435 -g

```

4. Quick One-Liner (No Script Needed)

If you ever just want to test it instantly without a script, you can run this single `curl` command in your terminal:

```bash
curl -s http://127.0.0.1:11434/api/tags | grep -o '"name":"[^"]*"'

```
