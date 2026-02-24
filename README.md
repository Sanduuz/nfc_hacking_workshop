# NFC Hacking Workshop

This repository contains materials from an NFC hacking workshop held for HelSec on 2026-01-29.

The slides used in the workshop can be found in [helsec_nfc_hacking_workshop_2026-01-29.pdf](./helsec_nfc_hacking_workshop_2026-01-29.pdf).

See below for setting up the hands-on challenges.

---

## Table of Contents
* [Hardware](#hardware)
* [Reader Simulators](#reader-simulators)
  1. [Dependencies](#dependencies)
  2. [Readers](#readers)
  3. [Reader Status Indicator](#reader-status-indicator)
* [Hands-on challenges](#hands-on-challenges)
  1. [Low Frequency Tag](#low-frequency-tag)
  2. [MIFARE Classic 1K](#mifare-classic-1k)
  3. [MIFARE DESFire EV1](#mifare-desfire-ev1)
  4. [MIFARE Ultralight EV1](#mifare-ultralight-ev1)
  5. [MIFARE Ultralight C](#mifare-ultralight-c)

---

## Hardware

The following list contains hardware used in the workshop. This gives you an idea on what hardware is required to set up the challenges locally:

- Proxmarks (At least 1, can be e.g. RDV4/EZ)
- ACR122U USB NFC Reader (based on PN532 chip)
- Cards
  - T5577
  - Mifare Classic 1K
  - Mifare DESFire EV1
  - Mifare Ultralight EV1
  - Mifare Ultralight C

---

## Reader Simulators

The reader simulators are made of 2 parts:
 - Readers
 - Reader Status Indicator

### Dependencies

- [rust](https://rust-lang.org/)
- [uv](https://docs.astral.sh/uv)

### Readers

Most of the readers in the workshop were built on top of proxmark and used the experimental client lib to interact with cards.

There was only one challenge (Mifare Classic 1K) where another reader (ACR122U) was used instead of a proxmark. This reader utilizes [libnfc](https://github.com/nfc-tools/libnfc) with `acr122u_usb` mode (no PCSC!)

**Proxmark experimental client lib**

This document will not focus on building the proxmark firmware/client software, but below is a Quick Start Guide for building the experimental lib (Proxmark RDV4).

For problems regarding proxmark software, see the [original GitHub page](https://github.com/rfidresearchgroup/proxmark3).

Quick Start Guide:

```
git clone https://github.com/rfidresearchgroup/proxmark3
cd proxmark3/client/
./experimental_lib/01make_lib.sh
cp ./experimental_lib/build/libpm3rrg_rdv4.so /path/to/reader/_pm3.so
cp ./pyscripts/pm3.py /path/to/reader/pm3.py
```

_Note: Even though the client lib is bundled in the repository you might need to rebuild it to match the firmware version installed in your proxmark._

**Starting the readers**

To run a proxmark reader:

- Make sure you have built the reader status indicator package (see [Reader Status Indicator](#reader-status-indicator) for more details).
- Go to reader directory: `cd /path/to/reader`
- Create uv virtual environment (if doesn't exist): `uv venv`
- Install reader_status_indicator package: `uv pip install /path/to/reader_status_indicator/target/wheels/wheel_file.whl`
- Run reader: `uv run reader.py`

_Note: Be sure to use absolute paths when installing reader_status_indicator to avoid venv confusion._

For Mifare Classic 1K reader (no proxmark):
- `cargo run --release`

If building fails you might need to add the following contents to `.cargo/config.toml` (Tested on Debian 13):

```toml
[env]
BINDGEN_EXTRA_CLANG_ARGS = """
-D__float128=long double
-I/usr/include/x86_64-linux-gnu
-I/usr/include
-I/usr/lib/llvm-19/lib/clang/19/include
-target x86_64-pc-linux-gnu
"""
```

### Reader Status Indicator

The reader status indicator is built in Rust and uses [winit](https://crates.io/crates/winit) and [wgpu](https://crates.io/crates/wgpu) to create a fullscreen window and renders colors in it.

The reader status indicator uses [PyO3](https://crates.io/crates/pyo3) to create Rust bindings for the Python interpreter. The reader simulator software, that uses proxmark experimental lib, calls the rust functionality to e.g. create a window, change the window color and close the window.

**Building and installing**

One can test the reader status indicator by running the following commands:

- Change directory to reader_status_indicator: `cd reader_status_indicator`
- Build and install into a local virtual environment: `uv run maturin develop --uv --release`
- Run demo.py in the same virtual environment: `uv run demo.py`

The package must be built and installed in order to use it in other virtual environments. To build it, run the following command: `uv run maturin build --release`

This will build and save the wheel file under `/target/wheels`.

---

## Hands-on challenges

The challenge descriptions and objectives are listed below, along with instructions for creating similar cards yourself. Every challenge included a Proxmark as a research tool, and some challenges also included a Flipper Zero.

The cards were created with a Proxmark3 running [_Iceman's firmware_](https://github.com/rfidresearchgroup/proxmark3). All commands under _Creating the card_ are Proxmark commands.

### Low Frequency Tag

**Description**

A T5577 chip was used in the workshop to emulate a Farpointe Pyramid-series tag. This tag used a facility code of 129 and a card number of 29126.

The challenge was to emulate this low frequency tag, effectively cloning it. This challenge also included a cheap Chinese lock that used a MIFARE Classic 1K HF card, but the lock validated only the 4-byte UID.

**Objective**

The objective of this challenge was to show that LF systems are typically low-security and easy to clone, and that using HF technology alone does not automatically make an access-control system secure.

**Creating the card**

- Write the correct facility code and card number: `lf pyramid clone --fc 129 --cn 29126`

### MIFARE Classic 1K

**Description**

This challenge had two parts. First, participants had to dump the card to find a flag in a sector protected by custom keys. Second, they had to load the dump into an emulator to clone the card. Cloning was verified by placing the emulator Proxmark on top of the reader simulator Proxmark and checking whether the screen turned green.

**Objective**

The objectives of this challenge were to examine how legacy proprietary cryptography (Crypto1) can lead to weaknesses, and to demonstrate how quickly full-card cloning becomes possible once keys are known.

**Creating the card**

- Write the authentication data read by the simulator: `hf mf wrbl --blk 4 -d 4461746155736564466F724175746800`
- Write the custom Key A (`43755A74304D`) for the authentication-data sector: `hf mf wrbl --blk 7 -d 43755A74304DFF078069FFFFFFFFFFFF`
- Write the flag:
  - Part 1: `hf mf wrbl --blk 60 -d 48454C5345435F43525950544F2D315F`
  - Part 2: `hf mf wrbl --blk 61 -d 49535F494E5345435552450000000000`
- Set the custom key A (`4E6F44656661`) and key B (`756C744B6579`) for the flag sector: `hf mf wrbl --blk 63 -d 4E6F44656661FF078069756C744B6579`

### MIFARE DESFire EV1

**Description**

The challenge was to reconstruct a flag from fragments distributed across multiple applications by ordering them using the values stored in value files.

**Objective**

The main objective of this challenge was to understand MIFARE DESFire memory organization and navigation across applications and files.

**Creating the card**

- Create the applications:
  - `hf mfdes createapp --aid f9ae22 --dstalgo 2TDEA`
  - `hf mfdes createapp --aid 548190 --dstalgo 2TDEA`
  - `hf mfdes createapp --aid eb8ca5 --dstalgo 2TDEA`
  - `hf mfdes createapp --aid 48bd05 --dstalgo 2TDEA`
  - `hf mfdes createapp --aid 8b1e2d --dstalgo 2TDEA`
  - `hf mfdes createapp --aid eb10ba --dstalgo 2TDEA`
  - `hf mfdes createapp --aid 0c8f31 --dstalgo 2TDEA`
  - This application contains a hint instead of a flag fragment: `hf mfdes createapp --aid 031337 --dstalgo 2TDEA`
- Create value files:
  - `hf mfdes createvaluefile --aid f9ae22 --fid 00 --lower 00000000 --upper 7fffffff --value 00000004 --cmode mac`
  - `hf mfdes createvaluefile --aid 548190 --fid 00 --lower 00000000 --upper 7fffffff --value 00000002 --cmode mac`
  - `hf mfdes createvaluefile --aid eb8ca5 --fid 00 --lower 00000000 --upper 7fffffff --value 00000000 --cmode mac`
  - `hf mfdes createvaluefile --aid 48bd05 --fid 00 --lower 00000000 --upper 7fffffff --value 00000006 --cmode mac`
  - `hf mfdes createvaluefile --aid 8b1e2d --fid 00 --lower 00000000 --upper 7fffffff --value 00000005 --cmode mac`
  - `hf mfdes createvaluefile --aid eb10ba --fid 00 --lower 00000000 --upper 7fffffff --value 00000003 --cmode mac`
  - `hf mfdes createvaluefile --aid 0c8f31 --fid 00 --lower 00000000 --upper 7fffffff --value 00000001 --cmode mac`
- Create standard files:
  - `hf mfdes createfile --aid f9ae22 --fid 01 --cmode mac --algo 2TDEA --size 000006 --rawrights e000 --amode mac`
  - `hf mfdes createfile --aid 548190 --fid 01 --cmode mac --algo 2TDEA --size 000005 --rawrights e000 --amode mac`
  - `hf mfdes createfile --aid eb8ca5 --fid 01 --cmode mac --algo 2TDEA --size 000007 --rawrights e000 --amode mac`
  - `hf mfdes createfile --aid 48bd05 --fid 01 --cmode mac --algo 2TDEA --size 000006 --rawrights e000 --amode mac`
  - `hf mfdes createfile --aid 8b1e2d --fid 01 --cmode mac --algo 2TDEA --size 000004 --rawrights e000 --amode mac`
  - `hf mfdes createfile --aid eb10ba --fid 01 --cmode mac --algo 2TDEA --size 000005 --rawrights e000 --amode mac`
  - `hf mfdes createfile --aid 0c8f31 --fid 01 --cmode mac --algo 2TDEA --size 000008 --rawrights e000 --amode mac`
  - This file is for a hint: `hf mfdes createfile --aid 031337 --fid 00 --cmode mac --algo 2TDEA --size 00005d --rawrights e000 --amode mac`
- Write flag fragments to standard files:
   - `hf mfdes write --aid f9ae22 --fid 01 --type data --data 5F6234353364`
   - `hf mfdes write --aid 548190 --fid 01 --type data --data 5F3768335F`
   - `hf mfdes write --aid eb8ca5 --fid 01 --type data --data 48454C5345435F`
   - `hf mfdes write --aid 48bd05 --fid 01 --type data --data 56616C753335`
   - `hf mfdes write --aid 8b1e2d --fid 01 --type data --data 5F6F6E5F`
   - `hf mfdes write --aid eb10ba --fid 01 --type data --data 464C34477A`
   - `hf mfdes write --aid 0c8f31 --fid 01 --type data --data 3072646572316E67`
- Write the hint: `hf mfdes write --aid 031337 --fid 00 --type data --data 4F726465722074686520666C616720667261676D656E747320696E207374616E646172642066696C6573206261736564206F6E2074686520696E6465782076616C756520676976656E206279207468652076616C75652066696C65732E`
- Create and configure an application used for authentication for the reader simulator:
  - Create an application: `hf mfdes createapp --aid 66556e --dstalgo 2TDEA`
  - Create an encrypted backup file: `hf mfdes createfile --backup --aid 66556e --fid 12 --amode encrypt --cmode mac --rrights key1 --wrights key1 --rwrights key1 --chrights key1 --size 00001f`
  - Change key 1: `hf mfdes changekey --aid 66556e --algo 2TDEA --newkeyno 1 --newkey 4b3379315f4630525f41757468316e47`
  - Write the authentication data to the backup file: `hf mfdes write --aid 66556e --fid 12 --data 316659307543346e5233346437686973506c336173654c33744d654b6e3077 -n 1 -k 4b3379315f4630525f41757468316e47`

### MIFARE Ultralight EV1

**Description**

The challenge was to sniff communication between the reader and card, recover the authentication password sent in plaintext, and then use that password to dump card memory and recover the flag from pages 4-9.

**Objective**

The objective of this challenge was to show the risk of transmitting credentials in plaintext and how a simple design flaw can be crucial.

**Creating the card**

- Write the flag:
  - Part 1: `hf mfu wrbl -b 4 -d 48454C53`
  - Part 2: `hf mfu wrbl -b 5 -d 45435F53`
  - Part 3: `hf mfu wrbl -b 6 -d 6E316666`
  - Part 4: `hf mfu wrbl -b 7 -d 696E675F`
  - Part 5: `hf mfu wrbl -b 8 -d 4630725F`
  - Part 6: `hf mfu wrbl -b 9 -d 50574473`
- Write the authentication data read by the simulator: `hf mfu wrbl -b 14 -d 8A27C6BF`
- Set protection to start from page 4: `hf mfu wrbl -b 16 -d 00000004`
- Require password for reading: `hf mfu wrbl -b 17 -d 80050000 -k FFFFFFFF`
- Set password (`4747455A`): `hf mfu wrbl -b 18 -d 4747455A -k FFFFFFFF`

### MIFARE Ultralight C

**Description**

This challenge focused on sniffing communication between the reader and card, then decoding the flag directly from the trace.

**Objective**

The main objective of this challenge was to show that communication after authentication remains plaintext, even when authentication itself uses the three-pass protocol.

**Creating the card**

- Write the flag:
  - Part 1: `hf mfu wrbl -b 14 -d 48454C53`
  - Part 2: `hf mfu wrbl -b 15 -d 45435F43`
  - Part 3: `hf mfu wrbl -b 16 -d 306D6D7A`
  - Part 4: `hf mfu wrbl -b 17 -d 5F696E5F`
  - Part 5: `hf mfu wrbl -b 18 -d 506C3469`
  - Part 6: `hf mfu wrbl -b 19 -d 6E373378`
  - Part 7: `hf mfu wrbl -b 20 -d 373F3F3F`
- Write the authentication data read by the simulator:
  - Part 1: `hf mfu wrbl -b 30 -d 34553748`
  - Part 2: `hf mfu wrbl -b 31 -d 55733344`
  - Part 3: `hf mfu wrbl -b 32 -d 4630724D`
  - Part 4: `hf mfu wrbl -b 33 -d 46554C43`
- Set the key: `hf mfu setkey --key 43757374306D5F332D4445535F4B6579`
