---
title: "The court's library, not its king"
description: "Maudit is a library, not a framework"
author: The Maudit Team
date: 2025-10-09
---

The average static site generator (SSG) works like this:

- You install some sort of package or binary in your project (ex: `astro`, `@11ty/eleventy`, `gatsby`, etc) or globally (ex: `hugo`, `zola`)
- You project may contain some sort of special folders, `src/pages`, `_includes`, etc. Or at least, has a convention for how to define your pages (which might just be a bunch of `.md` files or configuration based, that's ok)
- Oftentimes, you'll have a config file (`gatsby.config.js`, `astro.config.js`, `hugo.toml` etc) to define some settings.

Then, to build your website you'll run a `build` command, running an internal build pipeline of the framework, transforming your pages, configuration etc, into nice `.html` files. Put more bluntly, you provide things to the framework, it does some things with it and so it goes. Great, nice.

Maudit instead provides an alternative model: **You call Maudit, it does not call you.** A Maudit project is a Rust project, you generate your website by running your normal Rust project's generated binary that uses Maudit as a library and call its methods.

## The Mantle is Thine

Pages in Maudit projects are just normal Rust structs. You can import them, give them properties, and implement methods. In other SSGs, getting the HTML of a single page is sometimes impossible, but in Maudit, you can just import the page and call its [`.build()`](https://docs.rs/maudit/latest/maudit/route/trait.FullRoute.html#method.build) method. It works!

This applies to all of Maudit. To render remote Markdown using Maudit's pipeline (with syntax highlighting, components, shortcodes, etc.), import [`render_markdown`](https://docs.rs/maudit/latest/maudit/content/markdown/fn.render_markdown.html) and pass your content and options. The code on the home page uses [`highlight_code`](https://docs.rs/maudit/latest/maudit/content/fn.highlight_code.html), the same function that `render_markdown` uses.

You can even build your own Maudit at home using these APIs, we have [a guide for that](https://maudit.org/docs/library/)! For most users, the [`coronate()`](https://docs.rs/maudit/latest/maudit/fn.coronate.html) function (the ["entrypoint"](https://maudit.org/docs/entrypoint/)) serves both as the standard way to build pages, bundle assets, and process images and the reference implementation for someone to learn from.

## For What Quest We Stand Here

These needs might seem niche, but it was actually born out of real pain points we've hit in current offerings. Most notably, for our own projects we often ran into the need to have access to the structured content the framework provide, but outside of the framework's blessed paths which proved to be cumbersome or sometimes totally impossible.

In Maudit, it is totally possible to use [content sources](https://maudit.org/docs/content/) outside of pages or outside the site's generation totally, it's cool! (we think!)

## Tis but a Scratch

Of course, this comes with some trade-offs. As a library, it's hard to provide the experience of "Just drop a few Markdown files in a folder and you're done!". You do need to know some Rust to get started, and you do need to set up your project a bit more manually than with other SSGs at times.

That said, we think that's okay. We'll do our best to make Maudit as friendly as we can, and the docs as clear and welcoming as possible. But if you end up needing something simpler to get going with, that'd be totally understandable.

---

We're very excited to see what people build with Maudit! Hopefully, the flexibility of Maudit empowers and motivate you to create websites that fits your exact and precise needs.

If you have any questions, feedback, or just want to say hi, feel free to [join our Discord](https://maudit.org/discord) or [open an issue or discussion on GitHub](https://github.com/bruits/maudit)!
