"""Test the running Tauri backend's OpenAI endpoint directly."""
import json, urllib.request, urllib.error

# Read running backend info from .env
with open('.env') as f:
    env = {}
    for line in f:
        line = line.strip()
        if not line or line.startswith('#') or '=' not in line:
            continue
        k, _, v = line.partition('=')
        env[k.strip()] = v.strip()

url = env.get('REST_API_URL', '')
token = env.get('REST_API_TOKEN', '')
print(f"Backend URL: {url}")
print(f"Token: {token[:12]}...")
print()

# Test OpenAI chat via backend
print("Testing /agent/openai/chat via backend...")
data = json.dumps({"message": "Say hi in 3 words", "model": "gpt-4o-mini"}).encode()
req = urllib.request.Request(
    url + "/agent/openai/chat",
    data=data,
    headers={
        "Authorization": "Bearer " + token,
        "Content-Type": "application/json",
    },
)
try:
    with urllib.request.urlopen(req) as resp:
        result = json.loads(resp.read())
        print("SUCCESS!")
        print("  Response:", result.get("response", "???"))
except urllib.error.HTTPError as e:
    body = e.read().decode()
    print(f"FAILED (HTTP {e.code}):")
    print(f"  {body[:500]}")
except Exception as e:
    print(f"ERROR: {e}")
