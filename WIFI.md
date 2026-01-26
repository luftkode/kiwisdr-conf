# WiFi Setup
## Building drivers from source
### Supported environment:
#### System:
```
root@kiwisdr:~# uname -r
5.10.168-ti-r82
root@kiwisdr:~# uname -m
armv7l
root@kiwisdr:~# hostnamectl
   Static hostname: kiwisdr
         Icon name: computer-embedded
           Chassis: embedded
        Machine ID: 5436a1d97ac646fe91d742fc5430cb34
           Boot ID: 8f58202d93614397aa46e99a59bf8e1c
  Operating System: Debian GNU/Linux 11 (bullseye)
            Kernel: Linux 5.10.168-ti-r82
      Architecture: arm
```
#### WiFi card:
 - [TL-WN722N v2/v3 (Realtek RTL8188EU)](https://www.dustin.dk/product/5011024916/tl-wn722n-usb-wifi-adapter)
### Guide
#### 1. Install dependencies
```bash
apt update
apt install -y build-essential dkms git linux-headers-$(uname -r)
```
#### 2. Clone the repo
```bash
cd /usr/src
git clone https://github.com/aircrack-ng/rtl8188eus.git
mv rtl8188eus realtek-rtl8188eus-5.3.9~20221105
cd realtek-rtl8188eus-5.3.9~20221105
```
#### 3. Blacklist old drivers
```bash
echo 'blacklist r8188eu' | sudo tee -a '/etc/modprobe.d/realtek.conf'
echo 'blacklist rtl8xxxu' | sudo tee -a '/etc/modprobe.d/realtek.conf'
```
#### 4. Stop `kiwid` (for faster compile times)
```bash
systemctl stop kiwid
```
#### 5. Add to `dkms` and build
> [!IMPORTANT]
> This may take 20 min
```bash
dkms add .
dkms build realtek-rtl8188eus/5.3.9~20221105
dkms install realtek-rtl8188eus/5.3.9~20221105
```
> [!NOTE]
> The compiled driver now lives here: `/lib/modules/$(uname -r)/updates/dkms/8188eu.ko`
#### 6. Reboot
```bash
reboot
```
## Using WiFi
#### 1. Install `connman`
```bash
apt install connman
```
#### 2. Open the connman` shell
```bash
connmanctl
```
#### 3. Scan wifi networks
```connmanctl
enable wifi          # Powers on the USB dongle
scan wifi            # Scans for wifis (Wait for "Scan completed" message)
services             # List available networks
```
Find your network's Service ID (it looks like `wifi_xxxxxx_managed_psk`) and copy it 
#### 4. Connect
```connmanctl
agent on                                # Enables password entry
connect <PASTE_YOUR_SERVICE_ID_HERE>    # Use Tab to autocomplete
# Enter your WiFi password when prompted
quit
```
#### 5. Test connection
```bash
ip a
```
