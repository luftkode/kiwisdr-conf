# Overview  
This repository installs and configues:  
* Nginx as a webserver for static files, proxy for OpenWebRX and Backend, and TLS middleware.  
* [KiwiClient](https://github.com/jks-prv/kiwiclient) for recording from the KiwiSDR.  
* Rust + Actix Web as a webserver for exposing KiwiClient as a REST api.  
* Html + CSS + Typescript as a web based frontend to KiwiClient.  
### Compatability
Oficial KiwiSDR firmware .img files are [here](http://kiwisdr.com/quickstart/index.html#id-dload).  
Only works with Debian 11 based KiwiSDR firmware, not Debian 9 based.  
If your KiwiSDR is using Debian 9, you must update.
This repo is known to work with
* KiwiSDR 1 + BeagleBone Green + [This firmware](http://kiwisdr.com/files/KiwiSDR_v1.804_BBG_BBB_Debian_11.11.img.xz) (SHA256: `2f60798f60b647f0b18f8ac7493776c7b75f22f17977dffdd6c8253274538c3f`)
* KiwiSDR 2 + BeagleBone Green + Stock firmware

# Installation Instructions  
### Connect with `ssh` and change password:
ssh into the Kiwi (from a terminal on your laptop)
```shell
ssh debian@kiwisdr.local
```
Default password is `temppwd`  
Switch user
```bash
sudo su
```
#### Change Passwords
Root password
```bash
passwd
```
Debian password
```bash
passwd debian
```
### (Optional) Setup ssh keys  
Generate a key (if you don't have one) (on your laptop, not the kiwi)  
```bash
ssh-keygen -t ed25519 -C "email@example.com"
```
Setup ssh key for `debian` user  
From linux:
```bash
ssh-copy-id debian@kiwisdr.local
```
From windows:
```powershell
type $env:USERPROFILE\.ssh\id_ed25519.pub | ssh debian@kiwisdr.local "cat >> .ssh/authorized_keys"
```
#### (Optional) Setup root ssh keys  
Allow direct root login over ssh (on the Kiwi)  
```bash
sudo sed -i '/^#\?PermitRootLogin/c\PermitRootLogin yes' /etc/ssh/sshd_config && 
sudo systemctl restart ssh
```
Transfer the key (on your laptop)
```bash
ssh-copy-id root@kiwisdr.local
```
After you have transferred the key you can disable direct root login with a password (so it only allows direct root login with ssh keys) (on the Kiwi) 
```bash
sudo sed -i '/^#\?PermitRootLogin/c\PermitRootLogin prohibit-password' /etc/ssh/sshd_config && 
sudo systemctl restart ssh
```
### Admin panel password  
Go to the KiwiSDR Admin Panel:  
[http://kiwisdr.local:8073/admin](http://kiwisdr.local:8073/admin)  (Note: `http`, not `https`)  
Go to the `Security` tab  
Edit `Admin password`  

### Add key
Add public.key to your keyring:
```bash
curl -fsSL https://raw.githubusercontent.com/Ultraegern/kiwisdr-conf/refs/heads/main/public.key | gpg --import
```
Mark the key as trusted (Only if you actualy trust the key):
```bash
gpg --import-ownertrust <<< "846475029CE00982F700C9AC3CB2F77A8047BEDC:3:"
```
> ⚠️ **Warning:** Only mark a key as trusted if you trust the person the key belongs to, and that the key is actually that person's key (eg. somebody hacked Github and replaced the key with their key).

### Install
Download the repository and run setup.sh:
```bash
curl -fsSL https://github.com/Ultraegern/kiwisdr-conf/archive/refs/heads/main.zip -o /tmp/kiwisdr-conf.zip && \
sudo apt install unzip -qq 1>/dev/null && \
unzip -qq /tmp/kiwisdr-conf.zip -d /tmp/ && \
rm /tmp/kiwisdr-conf.zip && \
cd /tmp/kiwisdr-conf-main && \
gpg --verify setup.sh.asc setup.sh 2>/dev/null && \
sudo chmod +x setup.sh && \
sudo ./setup.sh
```

Now you can go to [https://kiwisdr.local/help](https://kiwisdr.local/help)
> ℹ️ **Note:** The TLS cert is Signed by KiwiCA. If you want the KiwiCA certificate run:  
>  ```bash
> scp debian@kiwisdr.local:/etc/ssl/kiwisdr/ca/KiwiCA.pem ./
>  ```

## Customise  
Go to the [Admin Panel](https://kiwisdr.local/admin)  
Go to the `Webpage` tab  
Top bar title:
```
KiwiSDR by SkyTEM Surveys ApS
```
Owner info: [Copy this file](https://github.com/Ultraegern/kiwisdr-conf/blob/main/header.html)  
Grid square: Continuous update from GPS: `true`  
Location: Continuous update from GPS: `Hi Res`  
Admin email:
```
it@skytem.com
```
