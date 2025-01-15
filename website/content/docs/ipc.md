---
title: "IPC Connection"
section: "advanced"
---

Every Maudit project's binary built using the `ipc` feature (enabled by default) includes an IPC (Inter-process communication) connection that can be used to communicate with the build process from other processes. For instance, you could use the IPC connection to trigger a rebuild of your project when a web server on the same machine receives a certain request.

Internally, this feature powers the `maudit dev` command, which watches the filesystem for changes and triggers a rebuild when necessary.

In the future, Maudit may support other ways to communicate with the build process, such as a TCP connection or a HTTP server.
