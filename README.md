<h1 align="center">
	<img src="./assets/app_icon.png" width="100" alt="Icon"/><br/>
	Fast Forward
</h1>

<p align="center">
	A window switcher for macOS built in Rust using the <a href="https://www.gpui.rs/">gpui</a> framework by <a href="https://zed.dev/">Zed</a>
</p>

![image](https://github.com/user-attachments/assets/9f7d272d-9244-46e8-afc8-debfc4d1261e)

> [!NOTE]
> This application was developed as a personal learning project to explore and understand Rust programming concepts. While functional, it may contain bugs or unexpected behaviors. I am not responsible for missed meetings because you got too efficient, carpal tunnel from excessive app switching, or your cat learning to manipulate your windows after watching you use this tool

## How It Works

1. Press the right Command key to trigger Fast Forward (or left Command key + Tab if enabled in settings).
2. You can:
   - Type to search for the app you want.
   - Use Tab to highlight the next app in the list.
4. Release the Command key to switch to the selected app.
   - To hide or close the selected app, press Space or Escape instead of releasing the Command key.

## Installation
1. Download the latest available DMG from releases page
2. Open the downloaded DMG file
3. Drag and drop the "Fast Forward.app" file to the Applications folder

You may see this message:
```
Apple could not verify "FastForward.dmg" is free of malware that may harm your Mac or compromise your privacy.
```

To resolve the problem, run the following command in Terminal:
```
xattr -d com.apple.quarantine ~/Downloads/FastForward.dmg
```
