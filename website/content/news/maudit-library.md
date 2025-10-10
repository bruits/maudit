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

Pages in Maudit projects are normal Rust structs, you can import them in other files, provide them properties, implement methods on them etc. To get the HTML of a single page in SSG frameworks is sometimes impossible, but in Maudit, you can import the page and call [`.build()`](https://docs.rs/maudit/latest/maudit/route/trait.FullRoute.html#method.build), it straight up just works!

This philosophy applies to all of Maudit, for instance to render remote Markdown using Maudit's Markdown pipeline (providing syntax highlighting, components, shortcodes, etc), you import [`render_markdown`](https://docs.rs/maudit/latest/maudit/content/markdown/fn.render_markdown.html) and pass it some content and options. The code on the homepage is highlighted using [`highlight_code`](https://docs.rs/maudit/latest/maudit/content/fn.highlight_code.html), which is the exact same function that `render_markdown` will use for highlighting as well, and so on.

It is intended for it to be possible to build your own static website generators based on all these APIs, [we even have on a guide on how to do it!](https://maudit.org/docs/library/), or if you're 99% of people, the [`coronate()`](https://docs.rs/maudit/latest/maudit/fn.coronate.html) function (the ["entrypoint"](https://maudit.org/docs/entrypoint/)) act as both the reference implementation of how to build pages, bundle assets, process images etc and also as the function the average Maudit project would call.

## For What Quest We Stand Here

These needs might seem niche, but it was actually born out of real pain points we've hit in current offerings. Most notably, for our own projects we often ran into the need to have access to the structured content the framework provide, but outside of the framework's blessed paths which proved to be cumbersome or sometimes totally impossible.

In Maudit, it is totally possible to use [content sources](https://maudit.org/docs/content/) outside of pages or outside the site's generation totally, it's cool! (we think!)

## Tis but a Scratch

Of course, this comes with some trade-offs. Being a library probably means it’s never going to feel as easygoing as something like Eleventy or Astro. You probably won’t be able to just toss a few Markdown files into a folder and call it a day. (One day, though, perhaps)

That said, we think that’s okay. We’ll do our best to make Maudit as friendly as we can, and the docs as clear and welcoming as possible. But if you end up needing something simpler to get going with, that’s fine too. No hard feelings.

---

We’re very excited to see what people build with Maudit! Hopefully, the flexibility of Maudit empowers and motivate you to create websites that fits your exact and precise needs.

If you have any questions, feedback, or just want to say hi, feel free to [join our Discord](https://maudit.org/discord) or [open an issue or discussion on GitHub](https://github.com/bruits/maudit)!
