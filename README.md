
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
 - **/con** Fast and convenient connecting to servers by using a command (`/con usw4`).
 - Might add more later, also feel free to open PRs!

### Dependencies

 - **iptables**. Should be already available on most systems.

### Usage

Set up your `rotmguard.toml` config file.
Compile the program with `cargo build --release` (or download binary from [releases](https://github.com/PonasKovas/rotmguard/releases)) and run the resulting executable:

```sh
sudo ./target/release/rotmguard
```

You **need root privileges** to run this tool, because **iptables** requires them.

Once `rotmguard` is up and running, just start playing the game and it should be working. You can always check if you're connected through the `rotmguard` proxy by typing `/hi` command in-game.

### Binbows<sup>®</sup> support when?? 😄

The majority of the code in this tool is platform independent, but one crucial component (`iptables` traffic re-routing) is Linux-only as far as I'm aware. To support Windows you would need an alternative to that, I don't know, I am not planning to add Windows support, ever. If you want, you can do it though, I will accept PRs if they're working.

# Commands

```
/hi | /rotmguard
```
Shows a notification. (Useful to check if you're connected through the proxy)

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
/ap | /antipush
```
Toggles the anti-push cheat.

