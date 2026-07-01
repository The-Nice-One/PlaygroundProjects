# Non-Market Housing Simulation

A interactive graphical simulation of New York City non-market housing.

# Motive

During my Government and Economics class, I was tasked with creating a project that addresses one of the two given prompts; in my case, a presentation of a possible solution to the housing crisis in NYC. Given a 9-day time frame, I decided to use the opportunity to connect class topics to my programming hobby, which is one of my favorite things to do. This project has also been featured in a segment of CBS Class Act and presented at a community event. 

# Features

* 2D/3D rendering of NYC housing data using [Bevy](https://bevy.org/).
* Uses real-world data sourced from [NYC OpenData](https://data.cityofnewyork.us) and [OpenStreetMap](https://www.openstreetmap.org/).
* Flexible and modular OpenStreetMap feature rendering, including rails, bridges, runways, etc.
* PMTiles and MBTiles streaming support to allow WASM and native runtimes.
* Arcade-like simulation of NYC housing market with simplified budget.
* Graphical user interface for converting and building new housing and toggling settings.
* In-depth camera controls for navigating the 2D/3D scene.

# Preview

![previewNonMarketHousingSimulation](https://raw.githubusercontent.com/The-Nice-One/PlaygroundProjects/refs/heads/main/Miscellaneous/NonMarketHousingSimulation/NonMarketHousingSimulationPreview.png)

# Usage

This project contains multiple scripts to generate the resources needed from the datasets, and a Rust project that can be built and ran. You can checkout the live WASM version at [https://non-market-housing-simulation.pages.dev/](https://non-market-housing-simulation.pages.dev/)

To build and run the project fully locally the following pre-requisites are needed:

* [New York OpenStreetMap Data](https://download.geofabrik.de/north-america/us/new-york.html).
* [PLUTO Dataset](https://data.cityofnewyork.us/City-Government/Primary-Land-Use-Tax-Lot-Output-PLUTO-/64uk-42ks/data_preview) 
* [NYC Building Footprints Dataset](https://data.cityofnewyork.us/City-Government/BUILDING/5zhs-2jue/about_data)
* [Tilemaker CLI](https://tilemaker.org/)
* [PMTiles CLI](https://docs.protomaps.com/pmtiles/cli)
* [Rust toolchain](https://www.rust-lang.org/tools/install)

Ideally, all downloads above should be in the same directory, with datasets being under a `data/` subdirectory.  
Begin by generating the `.mbtiles` file from the `.osm.pbf` with:
```bash
tilemaker data/new-york-260512.osm.pbf --output res/new-york.mbtiles --config scripts/config-openmaptiles.json --process scripts/process-openmaptiles.lua
```

You can then convert the `.mbtiles` file to a `.pmtiles` file with:
```bash
pmtiles convert res/new-york.mbtiles res/new-york.pmtiles
```

Generate the `lot_lookup.bin.gz` file with:
```bash
python scripts/build_lot_lookup.py ./data/BUILDING_20260513.csv ./data/pluto_25v4.csv ./res/lot_lookup.bin.gz
```

Edit line 25 of [src/main.rs](https://github.com/The-Nice-One/PlaygroundProjects/blob/main/Miscellaneous/NonMarketHousingSimulation/src/main.rs):
```diff
- pub const RUN_LOCAL: bool = false;
+ pub const RUN_LOCAL: bool = true;
```

Finally, run the project via Cargo:
```bash
cargo build --release

cargo run --release
```

# License

As with all other projects in this playground, the license is CC BY-NC.
