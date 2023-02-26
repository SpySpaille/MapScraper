
# Map Scraper

MapScraper is a Source Engine script that extracts a map's content, such as models, textures, sounds, scripts, etc.


## Installation
[![AGPL License](https://img.shields.io/github/downloads/SpySpaille/MapScraper/total)](http://www.gnu.org/licenses/agpl-3.0)

- Download the latest release **[here](https://github.com/SpySpaille/MapScraper/releases/latest)**
- Place the executable in a **dedicated** folder
- Execute the executable file

## Usage/Examples

Simple exemple:
```console
$ - 📂 Game Path > C:\Steam\steamapps\common\GarrysMod\garrysmod
$ - 🗒️ VMF File > C:\Steam\steamapps\common\GarrysMod\garrysmod\maps\gm_construct.vmf
$ - 🧱 Extract Materials (y/n) > y
$ - 📐 Extract Models (y/n) > n
$ - 🔊 Extract Sounds (y/n) > y
```
Output:
```console
$ - ✅ 33 materials have been extracted to the output folder
$ - ✅ 12 sounds have been extracted to the output folder
```
All extracted content will be in the folder named "output", make sure you have placed the executable in a dedicated folder to see the output folder.

## Features

- Extract materials and textures from VMF file.
- Extract 3D Models files and materials from VMF file.
- Extract sounds files from VMF file.


## Author

- [@SpySpaille](https://github.com/SpySpaille)

