"""
Simple OpenAI API test - reads OPENAI_API_KEY from .env
Tests: key validity, model listing, and a minimal chat completion.
"""

import os
import json
import urllib.request
import urllib.error

# ---- Load .env manually (no dependencies needed) ----
def load_dotenv(path=".env"):
    if not os.path.exists(path):
        print(f"ERROR: {path} not found")
        return
    with open(path) as f:
        for line in f:
            line = line.strip()
            if not line or line.startswith("#") or "=" not in line:
                continue
            key, _, value = line.partition("=")
            os.environ[key.strip()] = value.strip()

load_dotenv()

API_KEY = os.environ.get("OPENAI_API_KEY", "")
if not API_KEY:
    print("ERROR: OPENAI_API_KEY not found in .env")
    exit(1)

print(f"API Key: {API_KEY[:12]}...{API_KEY[-4:]}")
print(f"Key length: {len(API_KEY)} chars")
print()

HEADERS = {
    "Authorization": f"Bearer {API_KEY}",
    "Content-Type": "application/json",
}

def api_request(method, url, data=None):
    """Make an HTTP request to OpenAI API."""
    body = json.dumps(data).encode() if data else None
    req = urllib.request.Request(url, data=body, headers=HEADERS, method=method)
    try:
        with urllib.request.urlopen(req) as resp:
            return json.loads(resp.read()), resp.status
    except urllib.error.HTTPError as e:
        error_body = e.read().decode()
        try:
            error_json = json.loads(error_body)
        except:
            error_json = {"raw": error_body}
        return error_json, e.code

# ---- Test 1: List models ----
print("=" * 50)
print("TEST 1: List Models")
print("=" * 50)

result, status = api_request("GET", "https://api.openai.com/v1/models")

if status == 200:
    models = result.get("data", [])
    # Filter to chat-capable models
    chat_models = [m for m in models if any(m["id"].startswith(p) for p in ["gpt-", "o1-", "o3-", "o4-", "chatgpt-"])]
    chat_models.sort(key=lambda m: m["id"])
    
    print(f"✓ Success! Total models: {len(models)}, Chat-capable: {len(chat_models)}")
    print()
    print("Chat-capable models:")
    for m in chat_models:
        print(f"  • {m['id']}  (owned by: {m.get('owned_by', '?')})")
else:
    print(f"✗ Failed (HTTP {status})")
    print(json.dumps(result, indent=2))

print()

# ---- Test 2: Simple chat completion ----
print("=" * 50)
print("TEST 2: Chat Completion (gpt-4o-mini)")
print("=" * 50)

chat_data = {
    "model": "gpt-4o-mini",
    "messages": [
        {"role": "user", "content": "Say hello in exactly 5 words."}
    ],
    "max_tokens": 50,
    "temperature": 0.7,
}

result, status = api_request("POST", "https://api.openai.com/v1/chat/completions", chat_data)

if status == 200:
    reply = result["choices"][0]["message"]["content"]
    usage = result.get("usage", {})
    print(f"✓ Success!")
    print(f"  Model:    {result.get('model', '?')}")
    print(f"  Reply:    {reply}")
    print(f"  Tokens:   prompt={usage.get('prompt_tokens', '?')}, completion={usage.get('completion_tokens', '?')}, total={usage.get('total_tokens', '?')}")
else:
    print(f"✗ Failed (HTTP {status})")
    print(json.dumps(result, indent=2))


