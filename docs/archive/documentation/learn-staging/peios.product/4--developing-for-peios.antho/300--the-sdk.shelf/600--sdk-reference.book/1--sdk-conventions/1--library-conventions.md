---
title: Library conventions
description: The handful of conventions that hold across every function in every libpeios module — learn them once and the rest reads as intent.
---

libpeios has a small number of conventions that hold across *every* function in *every* module. They are deliberately uniform: once you know how one function reports an error or returns a variable-length buffer, you know how all of them do. This page is the one to read slowly. Everything else in this documentation assumes it.

The conventions come in four groups: **how results are returned**, **the two-call buffer protocol**, **memory ownership** (builders and views), and **the small stuff** (file descriptors and constants).
