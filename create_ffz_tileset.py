import requests
from PIL import Image
from io import BytesIO
import os
import sys

# Configuration
TOTAL_EMOTES = 1024
EMOTE_SIZE = 112 # Native resolution (standard Twitch/FFZ high res)
GRID_SIZE = 32 # 32x32 = 1024 emotes
# Final tileset will be (32 * 112) x (32 * 112) = 3584 x 3584 pixels
API_URL = "https://api.frankerfacez.com/v1/emoticons"
OUTPUT_PATH = "assets/ffz_tileset.png"

def fetch_emotes():
    emotes = []
    page = 1
    per_page = 200
    
    print(f"Fetching {TOTAL_EMOTES} most popular emotes from FFZ...")
    
    while len(emotes) < TOTAL_EMOTES:
        params = {
            "sort": "count-desc",
            "per_page": per_page,
            "page": page,
            "high_dpi": "on" # Enable high DPI to get the best urls
        }
        
        response = requests.get(API_URL, params=params)
        if response.status_code != 200:
            print(f"Error fetching page {page}: {response.status_code}")
            break
            
        data = response.json()
        page_emotes = data.get("emoticons", [])
        
        if not page_emotes:
            break
            
        for emote in page_emotes:
            if len(emotes) >= TOTAL_EMOTES:
                break
                
            # Pick the largest key available in urls (usually "4", then "2", then "1")
            urls = emote.get("urls", {})
            available_sizes = sorted([int(k) for k in urls.keys()], reverse=True)
            
            if not available_sizes:
                continue
                
            best_key = str(available_sizes[0])
            url = urls.get(best_key)
            
            if url:
                if url.startswith("//"):
                    url = "https:" + url
                emotes.append({
                    "id": emote["id"],
                    "name": emote["name"],
                    "url": url,
                    "resolution": best_key
                })
        
        print(f"  Found {len(emotes)} emotes...")
        page += 1
        
    return emotes[:TOTAL_EMOTES]

def create_tileset(emotes):
    tileset_dim = GRID_SIZE * EMOTE_SIZE
    print(f"Creating {tileset_dim}x{tileset_dim} tileset...")
    
    # Create transparent canvas
    canvas = Image.new("RGBA", (tileset_dim, tileset_dim), (0, 0, 0, 0))
    
    for i, emote in enumerate(emotes):
        row = i // GRID_SIZE
        col = i % GRID_SIZE
        
        try:
            resp = requests.get(emote["url"])
            if resp.status_code == 200:
                img = Image.open(BytesIO(resp.content)).convert("RGBA")
                
                # Resize if the image is larger than our target slot (LANCZOS for downsampling)
                # If smaller, we center it.
                if img.width > EMOTE_SIZE or img.height > EMOTE_SIZE:
                    img.thumbnail((EMOTE_SIZE, EMOTE_SIZE), Image.Resampling.LANCZOS)
                
                # Center the image in the EMOTE_SIZE x EMOTE_SIZE slot
                x_offset = (EMOTE_SIZE - img.width) // 2
                y_offset = (EMOTE_SIZE - img.height) // 2
                
                canvas.paste(img, (col * EMOTE_SIZE + x_offset, row * EMOTE_SIZE + y_offset), img)
            
            if (i + 1) % 100 == 0:
                print(f"  Processed {i + 1}/{len(emotes)} emotes...")
                
        except Exception as e:
            print(f"  Error processing emote {emote['name']} ({emote['id']}): {e}")
            
    return canvas

def main():
    if not os.path.exists("assets"):
        os.makedirs("assets")
        
    emotes = fetch_emotes()
    
    if not emotes:
        print("No emotes found.")
        return
        
    tileset = create_tileset(emotes)
    tileset.save(OUTPUT_PATH)
    print(f"Successfully saved tileset to {OUTPUT_PATH}")

if __name__ == "__main__":
    main()
