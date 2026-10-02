# Chinese-learner communities that will accept a beta-test request

**Research date:** 28 September 2026. Companion to
`GOOGLE_PLAY_TESTER_COMMUNITIES.md` (tester-swap venues and Google's policy).

Methods: `curl` HTTP/DNS checks, Discord's public invite API
(`discord.com/api/v10/invites/<code>?with_counts=true`), Reddit's archived-data
API (Arctic Shift) for subscriber counts and last-post dates, the Wayback Machine
for rule text, and `web_fetch` for forums. Where the rule text could not be read,
that is stated rather than guessed.

## Bottom line

- **Exactly one venue has a verified, sanctioned, actively-used place for "beta
  test my app" posts: Chinese-Forums → "Introduce Your Product Here".** There is
  a recent precedent post doing precisely this.
- **Reddit is hostile or inaccessible for this purpose.** r/ChineseLanguage's
  current rule text is unreadable from an automated client;
  r/languagelearning bans single-language posts and treats your own content as
  bannable; r/LearnMandarin says "No ads for your own stuff!" outright.
- **Three plausible-sounding subreddit names do not exist** (verified):
  r/ChineseLanguageLearning, r/Chinesetutor, r/AnkiChinese — and neither does
  r/HSK.
- **how-to-learn-any-language.com is dead** — the domain no longer resolves.
- **Discord has the largest audiences, but every rules channel is behind a
  join**, so no server's self-promotion policy could be verified. Ask a moderator
  in-server first.

## Tier 1 — post here

### Chinese-Forums → "Introduce Your Product Here"

<https://www.chinese-forums.com/forums/forum/73-introduce-your-product-here/>

Invision Community forum. Small but alive: 33 topics in the subforum, newest
September 21–22, 2026; overall pace "maybe 2 to 3 topics per week". Admin
`mikelove`, moderators `Lu` and `Moshen`.

**The policy is unambiguous** — "New Product Promotion Policy", posted by admin
mikelove 2026-07-20 and followed up 2026-07-31
(<https://www.chinese-forums.com/forums/topic/64543-new-product-promotion-policy/>):

> This will be the only permitted place for such commercial posts going forward.
> The one-introduction-post-per-product policy still applies, and we will be
> taking a generally dim view of attempts to squeeze commercial content into
> posts in other forums.

Asked directly whether free and open-source products are covered:

> At the moment, we're including free and even open-source products in this
> category too.

And:

> you should not be promoting your own product in posts outside of that
> "introduce your product" forum. Trying to induce a discussion of your product
> by asking leading questions about something related to it will also be frowned
> upon, and repeated attempts at tiptoeing around the edges of this policy may
> result in a ban.

Subforum description, read live: "This is the only forum such posts are allowed
in, and posts made here will not appear on the front page feeds."

Supporting rules, "Info for New Members"
(<https://www.chinese-forums.com/forums/topic/26753-info-for-new-members>):
"Advertising something. Ask first." New members have no PMs or profiles, posts
are checked before appearing until enough have been approved, and posts must be
"mostly in English".

Moderator Moshen, same period: product posts are removed when duplicated, empty
of content, or "not primarily about Chinese" — but "many first-time posters who
come here to let folks know about their new app have come up with something that
might be useful for Chinese learners… such posts are generally approved."

**Precedent:**

- <https://www.chinese-forums.com/forums/topic/64602-help-me-test-my-sentencegrammar-pattern-app/> — 16 September 2026, 957 views, still online.
- "I built an HSK 3.0 vocabulary app with Lock Screen widgets — looking for feedback", 27 June 2026, 5 replies.
- Pleco's own <https://www.chinese-forums.com/forums/topic/63086-pleco-40-beta-signups-live/>

**Audience:** English-language, overseas/expat skew with some members in China
and Taiwan; mixed Android and iOS; not mainland-majority, so Play is reachable.

**Approach:** register, then post **once** in that subforum, framed as a resource
and feedback request rather than an ad. Do not post it elsewhere, do not ask
leading questions elsewhere, do not bump it. Expect the first post to sit in a
moderation queue.

## Tier 2 — Discord, largest reach, rules unverified

Counts read from Discord's API on 2026-09-28. Every server keeps its rules behind
the join (`规则・rules`, `welcome`, `新手入門｜start-here`), so **nothing about
self-promotion could be verified — ask a moderator before posting.**

| Server | Invite | Members | Online |
| --- | --- | --- | --- |
| 中英交流 Chinese-English Language Exchange | <https://discord.gg/TJT6KAxp4U> (also `discord.gg/ADdR45y`) | 81,648 | 8,307 |
| r/ChineseLanguage's Discord | <https://discord.gg/G4nagat> | 44,762 | 4,468 |
| Yomitan | <https://discord.gg/YkQrXW6TXF> | 1,773 | 276 |
| Chinese (via DISBOARD) | <https://discord.gg/Zvt4WWPsQN> | 564 | 73 |
| I'm Learning Mandarin | <https://discord.gg/b9PAyKwT9p> | 536 | 30 |

These English-language servers skew US/EU/SEA and **Discord is blocked in
mainland China**, so members are overwhelmingly outside the mainland — which
makes this the lowest access-risk audience for a Google Play beta.

DISBOARD's own tags are not worth the effort: `chinese-learning` lists 3 servers
of 27–56 members and `hsk` lists 2. Listed numbers there are *online* counts, not
members.

## Tier 3 — Reddit, only via moderators

Reddit blocks automated access from this environment, so **counts and activity
are verified but current rule text mostly is not.**

| Subreddit | Subscribers | Status |
| --- | --- | --- |
| r/ChineseLanguage | 242,336 | Very active (last post 2026-09-28). Rules unreadable. Self-promotion is a long-contested, heavily moderated issue here: 2020 threads "The self-advertising on this sub is getting ridiculous" and "Can we do something about the self-promotion on this sub?", and a 2021 mod poll that the mods themselves voided for suspected vote manipulation. Sidebar points at curated `/wiki/Software` and `/wiki/resources` pages. Commercial posts appear in megathreads. |
| r/LearnChinese | 11,439 | **Restricted** — only approved users can submit. Its whole sidebar is a Discord link. |
| r/languagelearning | 3,101,951 | Rules verified: "Do not submit self-owned content too frequently… less than once a month", and "Do not post disallowed content… posts focused on one language". Submit page warns owners "may be banned without warning". Also: "What if my friend posted it for me? We treat this exactly the same as if the owner posted it themselves." **Not a venue for a Chinese app.** |
| r/LearnMandarin | 10,446 | Active, and the description says "**No ads for your own stuff!**" |
| r/MandarinChinese | 5,259 | Active, restricted posting. |
| r/Chinese_handwriting | 4,374 | Active (last post 2026-09-24), niche and exactly on topic. Rules unread. |
| r/WriteStreakCN | 1,339 | Writing corrections; not an app venue. |
| r/TestMyApp | 4,557 | Active, public, and *designed* for developer testing — but requires the title format "**[v0.5, Android]** …" and rule 10 says "No advertising apps. This is for development only." Non-conforming submissions are deleted. |
| r/androidapps | 363,340 | Very active; rules unretrievable. |
| r/Anki | 152,972 | Active; guidelines say "Do not spam. No research for other products/services" — a non-Anki app is off topic. |
| r/ChineseLearning | 9 | Dead (last post 2024-08). |
| r/Mandarin | 631 | Dead. |
| r/betatest | 182 | Effectively dead. |

**Do not post to these — verified non-existent:** r/ChineseLanguageLearning,
r/Chinesetutor, r/AnkiChinese, r/HSK, r/hsk, r/ChineseExams, r/AndroidBetaTesters.

**Approach:** modmail first, never a cold post. For r/ChineseLanguage the two
legitimate routes are asking the moderators whether a recruitment post is allowed
(or belongs in a megathread) and asking to be added to `/wiki/Software`. For
r/LearnChinese, go to its Discord or ask for approved-submitter status.

## Anki and Yomitan

- **forums.ankiweb.net** (plural — `forum.ankiweb.net` does not resolve) is up.
  Rules are stock Discourse boilerplate with no explicit self-promotion clause,
  and the forum API shows a history of developers posting their own
  **Anki-related** tools. A standalone non-Anki Chinese app is off topic; the
  viable angle is a deck or an Anki/Yomitan export.
- **Yomitan Discord** (<https://discord.gg/YkQrXW6TXF>, 1,773 members, hub
  <https://yomitan.wiki/>) is the popup-dictionary and Anki power-user crowd.
  Rules behind the join.

## Not viable, or cannot be verified

- **Pleco Forums** (<https://plecoforums.com/>) is very active — 6,594 threads,
  53,953 messages, 7,992 members, with separate iPhone/iPad and Android sections
  — but it is the vendor's own support forum and hosts Pleco's own beta. The only
  written rules are boilerplate banning "spam or spam-like" content; there is no
  explicit competitor clause, and none is claimed here. Practically, a competing
  Chinese-learning app would read as competitive advertising. Treat as not
  viable unless the owner (`mikelove`, the same person who administers
  Chinese-Forums) grants permission off-forum.
- **hackingchinese.com** is a blog, podcast, paid courses and a "Challenges"
  page — **not a verified community**. The Challenges page rendered no community
  rules or forum. Treat it as an editor contact
  (<https://www.hackingchinese.com/about/#contact>) and ask whether Olle Linge
  would mention the app.
- **forum.language-learners.org** — every automated request returned HTTP 405
  with a Cloudflare human-verification interstitial, and no usable Wayback
  capture exists. The domain is live and a search result suggests a 2025
  activity thread, but **size, activity and self-promotion rules are unverified.
  A human must look in a browser before this is treated as a venue.**
- **how-to-learn-any-language.com is dead.** `dig` returns no A record and curl
  fails to resolve the host, with or without `www`, on http and https. Its
  successor is the Language Learners' Forum above; the "Learn Any Language" wiki
  is a wiki, not a community.
- **Facebook groups are unverifiable without a logged-in account.** One real
  group was confirmed from its operator's own site —
  <https://facebook.com/groups/imlearningmandarin>, linked from
  <https://imlearningmandarin.com/join-the-im-learning-mandarin-facebook-community/>
  — but unauthenticated fetches returned only a title: no member count, no posts,
  no rules. Every group's size, activity and rules need a human in a browser, and
  "no advertising / admin approval required" should be assumed until read.

## Unreliable source, do not reuse

<https://monchinese.me/en/blog/cong-dong-online-tieng-trung-2026-pho-bien> — a
2026 SEO listicle of "10 online Mandarin learning communities". It gives
r/ChineseLanguage as "~150K" against the verified 242,336, and references a
vague "Discord 'Chinese Forum'" that matches no verified server.
