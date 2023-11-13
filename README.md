# Home Assistant Tether Agent addon

## Repo Structure
### hass-addon
This folder contains the addon that should be copied to the Home Assistant addons folder. When installing on a client, this is all you need to do to make the addon available to HA.

### image-src
This folder contains the source files for the Rust Home Assistant Tether Agent core utility (inside hass-tether-agent), and the necessary config files for dockerizing.


## How to update the addon
1. The Rust source files are found in hass-tether-agent. Make whatever changes are required in here and test using `cargo run`.
2. To update the distributed image file with the latest changes, in the `image-src` folder run the following:
* `docker build -t randomstudiotools/hass-tether-agent .`
* `docker image push randomstudiotools/hass-tether-agent`
3. Go to the addon page in Home Assistant, press `Rebuild`. The latest image will be pulled, mounted, and started.