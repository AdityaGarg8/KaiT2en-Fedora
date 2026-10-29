# T2 Share

Sends and receives files with Android phones through Quick Share, the
sharing feature built into Android. Phone and Mac have to be in the same
Wi-Fi. iPhones are not supported.

- Open T2 Share and switch on "Visible to nearby phones". This starts the
  user service `t2-share.service`, which also runs after the next login.
- On the phone, choose Share, then Quick Share, then the name of the Mac.
  The Mac shows up with its hostname.
- A phone that is not trusted yet raises a notification with the files, the
  PIN and the buttons Accept, Always trust and Decline. The PIN matches the
  one on the phone. Without an answer the transfer is declined after 60
  seconds.
- Files go to the Downloads folder, the window lets you choose another one.
  "Show in folder" in the notification opens the folder with the files
  selected.

Always trust accepts later transfers from that phone without asking. Quick
Share gives a phone no fixed identity outside of Google's contact feature,
so T2 Share recognizes a trusted phone by its device name only. Give your
phone a unique name that is hard to guess, on Android under Settings, About
phone, Device name. Trusted devices are listed in the window and can be
removed there.

To send, right-click files in the file manager and choose "Send with T2
Share" (Files, Dolphin, Nemo), use "Open With" in other file managers, or
drop files on the T2 Share window. The phone shows up once Quick Share on
it is visible to everyone and Bluetooth on the Mac is on, the Mac wakes the
phone through Bluetooth. Accept on the phone, both sides show the same PIN.
A phone set to contacts only appears as a hidden device.

If that is not secure enough for you, leave T2 Share switched off or stop
the service with

```bash
systemctl --user disable --now t2-share.service
```
