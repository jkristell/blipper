# Blipper

Blipper is a suite of tools for working with Infrared Remote Controls in Rust.
It's a companion software to my remote control encoding/decoding library
 [Infrared](http://github.com/jkristell/infrared) and I have been using it for
developing and testing that library

The firmware is developed for the Stm32 based Bluepill board. The host
communicates with the device over a serial port interface through USB.

## Commands
 - capture - Capture data from device, optionally write it to a .vcd file to
 for example look at the pulses in Pulseview.
 - decode - Capture data from device and run infrareds protocol decoders on host
 - playback-vcd - Use a vcd file as input to the decoders
 - transmit - send a ir command with the device

## TODO
This software is very much a work in progress, not even the TODO is done.

