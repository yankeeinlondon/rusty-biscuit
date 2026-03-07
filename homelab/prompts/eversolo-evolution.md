# Eversolo Streamer

## Power State Abstraction

The Eversolo Streamers A8 (but also A6 and likely A10 too) have only two true "power states": on and off.

- the only to wake the Eversolo from the "off" state is to send a magic Wake-on-LAN (WOL) packet to the device but to do this the UC Integration must be on the same subnet as the Eversolo.

Interesting the user's experience is a bit more nuanced with regard to power states. There is "in effect" a Standby mode too. After some unknown period of time where the Eversolo streamer has not played any media, it will enter this "standby" state by turning the front panel display off.

That means when we detect:

- no media playing
- AND, display is off

we should associate that as the device being in the Standby state.

## Changes

I believe that this logic is already in the Homelab Server but it is not currently in the `eversolo-integration` for Unfolded Circle.

- fix this in the eversolo-integration
- validate that it is implemented in the homelab server too
