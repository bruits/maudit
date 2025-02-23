---
title: "Philosophy"
description: "Maudit follows a few core principles that guide its development and design"
section: "getting-started"
---

### Maudit is about making static websites

Many of the modern web frameworks have gained new output modes opposite to their original purpose, for instance Next.js, a SSR-first framework, has `output: "export"` to generate a static website and Astro, SSG-first, has `output: "server"` to do the reverse.

While there is nothing intrinsically wrong with wanting to grow the use cases your software can serve, supporting different output modes comes with a inherent cost, both technological and human.

Supporting certain features in your less used output mode might add technical constraints affecting your main use case, and your attention is forever split between the two universes you intend to support.

**Maudit is about making static websites**. It has no higher goals than that. It won't try to become a server-side rendering framework, a hybrid framework, or anything else. This focus allows us to make the best static website generator we can.

## Your website changes less often than its content

Many parts of Maudit projects are written in Rust, a compiled language, thus requiring recompilation for changes. This overhead is justified by the assumption that structural changes are less frequent than content updates. For example, in a blog, new articles are more common than layout changes.

Without delving into the complexities of incremental builds, this architecture allows Maudit projects to build very quickly, even for large websites, thanks to the raw performance of a compiled language.
