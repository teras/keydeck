# 🎮 KeyDeck

> Stream Deck & friends for Linux - Button magic awaits! ✨

Whether you rock an Elgato Stream Deck, a budget-friendly Mirabox, or any button-filled friend in between, KeyDeck makes them all shine on Linux. Press buttons, trigger magic, and automate your way to glory! 🚀

## ✨ What's This?

KeyDeck is a lightweight daemon that makes your Elgato Stream Deck and compatible macro pads work beautifully on Linux. It comes with:

- **keydeck** - The daemon that runs in the background doing all the heavy lifting
- **keydeck-config** - A friendly GUI to configure your buttons and make them do cool stuff

## 🚀 Quick Start

### Installation

**Download the latest release:**
1. Head over to [Releases](https://github.com/teras/keydeck/releases)
2. Grab the `keydeck-{version}-linux.zip` file
3. Unzip it:
   ```bash
   unzip keydeck-*-linux.zip
   ```
4. Install the binaries:
   ```bash
   sudo cp keydeck keydeck-config /usr/local/bin/
   sudo chmod +x /usr/local/bin/keydeck /usr/local/bin/keydeck-config
   ```

### Running

⚠️ **Important:** Both `keydeck` and `keydeck-config` must be installed in the same directory (like `/usr/local/bin/`) for everything to work properly!

Once installed, just launch the configurator:
```bash
keydeck-config
```

The UI will automatically handle starting and managing the daemon for you. No need to manually run `keydeck` - the config app has your back!

Now click some buttons and make magic happen! ✨

## 🎯 Features

- 🖼️ **Custom button images** - Make your deck look exactly how you want
- ⌨️ **Keyboard shortcuts** - Automate all the things
- 🚪 **Application launching** - One button to rule them all
- 📄 **Multiple pages** - Organize buttons like a pro
- 🎨 **Templates** - Reuse your favorite button configs
- 🔄 **Dynamic content** - Buttons that update based on what you're doing
- 🎭 **X11 & KWin/Wayland support** - Works on both display servers (we play nice with everyone!)

## 🤝 Supported Devices

KeyDeck works with a wide variety of macro pads and Stream Deck devices:

**Elgato Stream Deck:**
- All Stream Deck models (via `elgato-streamdeck`)

**Ajazz:**
- AKP03, AKP03E (variants: 0x1002, 0x3002), AKP03R
- AKP153, AKP153E (variants: 0x1010, 0x3010), AKP153R
- AKP815

**Mirabox:**
- HSV293S
- HSV293SV3 (variants: 0x1005, 0x1014)
- N3, N3EN

**Mars Gaming:**
- MSD-ONE
- MSD-TWO

**Other Brands:**
- Mad Dog GK150K
- Redragon SS-551
- Risemode Vision 01
- Soomfon Stream Controller SE
- TMICE Stream Controller
- TreasLin N3

## 🙏 Credits

Special thanks to:
- [`elgato-streamdeck`](https://github.com/OpenActionAPI/rust-elgato-streamdeck) - For Elgato device support
- [`mirajazz`](https://github.com/viandoxdev/mirajazz) - For supporting more devices ([json-fork variant](https://github.com/teras/mirajazz/tree/json-fork))

---

*Made with ❤️ for the Linux community*
