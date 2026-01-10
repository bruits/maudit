---
title: "The still scrolls of the web, unchanging and steadfast, at last!"
description: "Maudit is about making static websites"
author: The Maudit Team
date: 2025-10-15
---

We have one goal for Maudit: To make it the best tool to generate static websites. This may include helpful side features like loading Markdown content, syntax highlighting, image processing, sitemap generation, RSS feeds, etc.

But the end result is always the same: You get a static website. No server, no serverless (with a server), no nothing. You get `.html` files that you can host wherever support hosting static files.

**Pros:**

- You can host Maudit websites pretty much anywhere, for typically cheap
- Hosting said websites is also headache-free and will last as long as servers are able to serve HTML files

**Cons:**

- You don't have a backend
- Static websites are typically quite static

There are many ways around both of these cons, typically involving hosting a backend or a serverless function and doing client-side fetches from your static website. This is a good solution, and work for many websites, but does still have limitations compared to a server-side rendering framework.

Ultimately..

## It's not you, it's us

**If you need server-side rendering: You probably shouldn't use Maudit.** You might counter this with "but, [Maudit is a library](/news/maudit-library/), it's so flexible, I can just render my Maudit pages in my Axum backend", which, great observation, but still, isn't the goal of Maudit!

You are free to do anything you want of course, but Maudit is not and won't be optimised for your use case, it might always feel like you are fighting against the library, building resentment against us, and generally having a not so good time.

## There is just so little time

We believe that it is fundamentally impossible for a tool to be great at both building static and dynamic websites. You can be good, but not great.

Maudit aims to absolutely delight people who want static websites. Adding server side rendering would introduce technical constraints and take development time away that would obstruct this goal.

You should use the proper tool for the job, and we hope for Maudit to be the proper tool for static websites.
