# Nightbot Now Playing

A Rust application that monitors Nightbot's song request queue and writes the currently playing song to a text file. Perfect for streamers who want to display the current song on their OBS overlays.

## Features

- Real-time monitoring of Nightbot song requests
- Automatically updates a text file with current song info
- Automatic OAuth token refresh
- Customizable song format

## Prerequisites

- A Nightbot account
- Nightbot API credentials (Client ID and Client Secret)

### Getting Nightbot API Credentials

1. Go to https://nightbot.tv/
2. Log in to your account
3. Navigate to https://nightbot.tv/account/applications
4. Click "New App"
5. Fill in the application details:
   - **Name**: Whatever you want (e.g., "Now Playing")
   - **Redirect URIs**: `http://localhost:5771` (or use a custom port)
6. Click "Submit"
7. Copy your **Client ID** and **Client Secret**

## Installation

### Option 1: Download Pre-built Binary (Recommended)

Download the latest release for your platform from the [Releases](../../releases) page.

### Option 2: Build from Source

1. Install Rust: https://rustup.rs/
2. Clone this repository:
   ```bash
   git clone <repository-url>
   cd nightbot-now-playing
   ```
3. Build the project:
   ```bash
   cargo build --release
   ```
4. The binary will be in `target/release/nightbot-now-playing` (or `.exe` on Windows)

## Setup

1. Place the binary in a folder where you want to run it
2. Create a `.env` file in the same folder as the binary
3. Add your Nightbot API credentials to the `.env` file:
   ```env
   CLIENT_ID=your_client_id_here
   CLIENT_SECRET=your_client_secret_here
   ```

### Example Folder Structure
```
your-app-folder/
├── nightbot-now-playing.exe  (or nightbot-now-playing on Linux/Mac)
├── .env
└── np.txt  (created automatically)
```

## Configuration

All configuration is done through the `.env` file. See `.env.example` for all available options.

### Required Configuration

| Variable | Description |
|----------|-------------|
| `CLIENT_ID` | Your Nightbot API Client ID |
| `CLIENT_SECRET` | Your Nightbot API Client Secret |

### Optional Configuration

| Variable | Default | Description |
|----------|---------|-------------|
| `REDIRECT_URI` | `http://localhost:5771` | OAuth redirect URI (must match what you set in Nightbot) |
| `FORMAT` | `{title} - {artist}, requested by {requester}` | Format string for song output |
| `POLL_INTERVAL` | `10` | How often to check for new songs (in seconds) |
| `OUTPUT_FILE` | `np.txt` | Name of the output text file |

### Format Variables

You can customize the output format using these variables:

- `{title}` - Song title
- `{artist}` - Artist name
- `{requester}` - Display name of the person who requested the song

**Examples:**
```env
FORMAT={title} by {artist}
FORMAT=🎵 {title} - {artist} (requested by {requester})
FORMAT=Now Playing: {title}
```

## Usage

1. Run the binary:
   ```bash
   ./nightbot-now-playing
   ```
   On Windows:
   ```cmd
   nightbot-now-playing.exe
   ```

2. On first run, a browser window will open asking you to authorize the application
3. Click "Authorize" to grant access
4. The browser will show "Authorization successful. You can close this window."
5. The application will now continuously monitor your Nightbot song queue
6. The current song will be written to `np.txt` (or your custom output file)

### Using in OBS

1. Add a **Text (GDI+)** source (or **Text (FreeType 2)** on Linux/Mac)
2. Check "Read from file"
3. Browse and select the `np.txt` file
4. Customize the font, color, and style as desired
5. The text will automatically update when a new song plays!

## Running on Startup (Optional)

### Windows

1. Press `Win + R`, type `shell:startup`, and press Enter
2. Create a shortcut to `nightbot-now-playing.exe` in this folder
3. Right-click the shortcut → Properties
4. Set "Start in" to the folder containing the binary and `.env` file

### Linux (systemd)

Create a systemd service file at `~/.config/systemd/user/nightbot-now-playing.service`:

```ini
[Unit]
Description=Nightbot Now Playing
After=network.target

[Service]
Type=simple
ExecStart=/path/to/nightbot-now-playing
WorkingDirectory=/path/to/folder-with-binary
Restart=always

[Install]
WantedBy=default.target
```

Enable and start:
```bash
systemctl --user enable nightbot-now-playing
systemctl --user start nightbot-now-playing
```

### macOS (launchd)

Create a plist file at `~/Library/LaunchAgents/com.nightbot.nowplaying.plist`:

```xml
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>Label</key>
    <string>com.nightbot.nowplaying</string>
    <key>ProgramArguments</key>
    <array>
        <string>/path/to/nightbot-now-playing</string>
    </array>
    <key>WorkingDirectory</key>
    <string>/path/to/folder-with-binary</string>
    <key>RunAtLoad</key>
    <true/>
    <key>KeepAlive</key>
    <true/>
</dict>
</plist>
```

Load it:
```bash
launchctl load ~/Library/LaunchAgents/com.nightbot.nowplaying.plist
```

## Troubleshooting

### "CLIENT_ID and CLIENT_SECRET must be set in .env"
- Make sure the `.env` file is in the same folder as the binary
- Check that your credentials are correctly entered without quotes

### "Failed to bind port"
- Another application might be using port 5771
- Change `REDIRECT_URI` to use a different port (e.g., `http://localhost:5772`)
- Make sure to update the redirect URI in your Nightbot application settings too

### "401 received, forcing refresh..."
- This is normal - the app will automatically refresh the token
- If it persists, delete `ACCESS_TOKEN`, `REFRESH_TOKEN`, and `EXPIRE_TIMESTAMP` from `.env` and restart

### Browser doesn't open for authorization
- Manually open the URL shown in the console
- Complete the authorization in your browser

### "No song currently playing" appears in the file
- This is normal when no song is in the queue
- Start a song request in your Nightbot to test

## Security Notes

- Keep your `.env` file private - it contains sensitive credentials
- Never commit your `.env` file to version control
- The access tokens are stored in `.env` and automatically refreshed

## Building from Source

Requirements:
- Rust 1.90 or later

```bash
cargo build --release
```

The binary will be in `target/release/`.

## License

MIT License - feel free to use this for personal or commercial projects.

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.
