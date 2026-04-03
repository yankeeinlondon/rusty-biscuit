# Google Maps Link

Generate a Google Maps URL for a given location.

## Approach

Plain Google Maps URL construction — no API key required. This uses the public `https://www.google.com/maps/@{lat},{lon},{zoom}z` or `https://www.google.com/maps/search/?api=1&query={lat},{lon}` format.

## API Surface

- Input: `Location` (or raw lat/lon)
- Output: URL string
- Sync — pure string formatting

## CLI

Integrated into other subcommand output (e.g., `where gps` or `where ip` could include a maps link in verbose mode), or available as a library utility.

## Key Considerations

- No API key or account needed for link generation
- Static Maps API (image generation) requires a key — out of scope
- URL format may include zoom level as optional parameter
