
from pathlib import Path



jsons = Path("demo/").glob("*.json")

with open("demo/demo.html", "w") as f:
    print('<script src="https://unpkg.com/@lottiefiles/lottie-player@latest/dist/lottie-player.js"></script>', file=f)
    print(file=f)
    print("""
        <style>
            lottie-player {
                width: 240px;
                height: 240px;
                display: inline-block;
            }
        </style>
        """, file=f)

    for filename in sorted([j.name for j in jsons]):
        print(f'<lottie-player autoplay loop mode="normal" src="./{filename}"></lottie-player>', file=f)
