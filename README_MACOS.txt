Plugin Installation
Download the latest plugin archive for your operating system (.zip files available on the releases page or after purchase).

macOS Installation
1. Extract the tonelab_vst_macos.zip archive (do not run it directly from inside the zip).
2. Right-click (or Control-click) the install_mac.command script, select Open, and then click Open again if a security warning appears.
3. The Terminal will open. Enter your Mac login password and press Return (the characters won't appear as you type, this is normal).
4. The script will automatically copy the plugin to your VST3 folder and remove macOS security blocks!
5. You can now close the Terminal and launch your DAW.

Alternative Manual Method:
If the script doesn't work, you can do it manually:
1. Copy the tonelab_vst.vst3 file.
2. Open Finder, press Cmd + Shift + G, type /Library/Audio/Plug-Ins/VST3/, and paste the plugin there.
3. Open Terminal and run: sudo xattr -rd com.apple.quarantine /Library/Audio/Plug-Ins/VST3/tonelab_vst.vst3
