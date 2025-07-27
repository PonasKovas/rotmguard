
<p align="center" style="font-size: 40px">
  <img src="assets/rotmguard.gif" /><br />
  <b>RotmGuard</b>
</p>

---

<p align="center"> Advanced autonexus tool for Linux (works with wine/proton)</p>

### Features

 - **Autonexus.** Keep in mind that even the best autonexus won't save your wizard if you're gonna goof around and walk over enemies that EP.
 - **Anti-Debuffs** Removes client-side debuffs such as confused, blind, etc.
 - **Fake-slow** Gives you a fake slow effect to help micro-dodge.
 - **Anti-Push** Disables ground pushing (such as conveyers in kogbolds or sprites).
 - **/con** Fast and convenient connecting to servers by using a command (`/con usw4`). Can only connect to Nexus.
 - **Damage Monitor** - also known as "sniffer", collect data about how much damage each player does to enemies (`/dmg` to generate a report).
 - **Notify** when loading a new level. Never miss the Castle in o3 again.
 - **Antilag** reduces client side lag (especially on older computers) by filtering out some unimportant packets.
 - Might add more later, also feel free to open PRs!

### Dependencies

 - **iptables**. Should be already available on most systems.

### Usage

Set up your `rotmguard.toml` config file.
Compile the program with `cargo build --release` (or download binary from [releases](https://github.com/PonasKovas/rotmguard/releases)) and run the resulting executable:

```sh
./target/release/rotmguard
```

You will need to enter the root password once you run this tool, because it needs to set up the **iptables** rules.

Once `rotmguard` is up and running, just start playing the game and it should be working. You can always check if you're connected through the `rotmguard` proxy by typing `/hi` command in-game.

# Commands

```
/hi
```

```
/rotmguard
```

Shows a notification. (Useful to check if you're connected through the proxy)

-------

```
/dmg
```
Generates and opens a damage report **of the current dungeon** in your browser.

```
/dmg 3
```
Generates and opens a damage report **of the last 3rd dungeon** in your browser.

```
/dmg sanc
```
Generates and opens a damage report **of the last dungeon containing the `sanc` substring** in your browser.

-------

```
/antilag
```
Toggles the antilag cheat.

-------

```
/devmode
```
Toggles the developer mode (shows client hp and a notification every time you take damage)

-------

```
/slow
```
Toggles the fake-slow cheat.

-------

```
/con <eue|eusw|use2|eun|use|usw4|euw2|a|uss3|euw|usw|usmw2|usmw|uss|usw3|ussw|usnw|aus>
```
Connects to the specified server.

-------

```
/autonexus [HP]
```
Shows the current autonexus HP value, or sets it. (Absolute value, not relative)

-------

```
/ap
```

```
/antipush
```

Toggles the anti-push cheat.

-------

```
/notify
```

Shows a desktop notification when a new level is loaded.
