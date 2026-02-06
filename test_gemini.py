#!/usr/bin/env python3
"""
Simple test script to verify Google Gemini API access.
Reads GEMINI_API_KEY from .env file or environment variable.

Usage:
    python test_gemini.py
    python test_gemini.py "Your custom message here"
"""

import os
import sys
import requests
from pathlib import Path

def load_env_file():
    """Load .env file and return dict of values"""
    env_path = Path(".env")
    env_vars = {}
    
    if env_path.exists():
        with open(env_path, 'r') as f:
            for line in f:
                line = line.strip()
                if line and not line.startswith('#') and '=' in line:
                    key, value = line.split('=', 1)
                    env_vars[key.strip()] = value.strip()
    
    return env_vars

def main():
    # Get message from command line or use default
    message = sys.argv[1] if len(sys.argv) > 1 else "Hello! What is 2 + 2?"
    
    print("=" * 50)
    print("Testing Google Gemini API")
    print("=" * 50)
    print()
    
    # Load API key
    env_vars = load_env_file()
    api_key = os.environ.get("GEMINI_API_KEY") or env_vars.get("GEMINI_API_KEY")
    
    if not api_key:
        print("ERROR: GEMINI_API_KEY not found!")
        print("Set it in .env file or as environment variable.")
        print()
        print("Get a key at: https://aistudio.google.com/app/apikey")
        sys.exit(1)
    
    if api_key == "YOUR_GEMINI_API_KEY_HERE":
        print("ERROR: GEMINI_API_KEY is still the placeholder value!")
        print("Replace it with your actual API key in the .env file.")
        print()
        print("Get a key at: https://aistudio.google.com/app/apikey")
        sys.exit(1)
    
    print(f"API Key: {api_key[:8]}...{api_key[-4:]}")
    print()
    print(f"Message: {message}")
    print()
    
    # Gemini API endpoint
    url = f"https://generativelanguage.googleapis.com/v1beta/models/gemini-2.0-flash:generateContent?key={api_key}"
    
    # Request payload
    payload = {
        "contents": [
            {
                "role": "user",
                "parts": [{"text": message}]
            }
        ]
    }
    
    print("Sending request to Gemini API...")
    print()
    
    try:
        response = requests.post(
            url,
            json=payload,
            headers={"Content-Type": "application/json"},
            timeout=30
        )
        
        print(f"Status Code: {response.status_code}")
        print()
        
        if response.status_code == 200:
            data = response.json()
            
            # Extract the response text
            if "candidates" in data and len(data["candidates"]) > 0:
                candidate = data["candidates"][0]
                if "content" in candidate and "parts" in candidate["content"]:
                    parts = candidate["content"]["parts"]
                    response_text = "".join(p.get("text", "") for p in parts)
                    
                    print("=" * 50)
                    print("Gemini Response:")
                    print("=" * 50)
                    print()
                    print(response_text)
                    print()
                    print("=" * 50)
                    print("SUCCESS! Gemini API is working correctly.")
                    print("=" * 50)
                else:
                    print("Unexpected response format:")
                    print(data)
            else:
                print("No candidates in response:")
                print(data)
        else:
            print("ERROR!")
            print()
            print("Response:")
            try:
                error_data = response.json()
                if "error" in error_data:
                    print(f"  Message: {error_data['error'].get('message', 'Unknown error')}")
                    print(f"  Status: {error_data['error'].get('status', 'Unknown')}")
                else:
                    print(error_data)
            except:
                print(response.text)
            
            if response.status_code == 400:
                print()
                print("This might mean your API key is invalid or the model name is wrong.")
            elif response.status_code == 403:
                print()
                print("This might mean your API key doesn't have access to the Gemini API.")
                print("Make sure you've enabled the Generative Language API in Google Cloud Console.")
            elif response.status_code == 429:
                print()
                print("Rate limit exceeded. Wait a bit and try again.")
                
    except requests.exceptions.Timeout:
        print("ERROR: Request timed out!")
        print("Check your internet connection.")
    except requests.exceptions.ConnectionError as e:
        print(f"ERROR: Connection failed!")
        print(f"  {e}")
        print("Check your internet connection.")
    except Exception as e:
        print(f"ERROR: {type(e).__name__}: {e}")

if __name__ == "__main__":
    main()
