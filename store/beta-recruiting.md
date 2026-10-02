# Recruiting the twelve testers

Companion to `tester-invite.md`, which covers the Console mechanics and the
tester's five steps. This file is about finding the people.

The venue research behind it is in
[docs/research/CHINESE_LEARNER_COMMUNITIES.md](../docs/research/CHINESE_LEARNER_COMMUNITIES.md)
(where Chinese learners actually gather, and what each place's rules say) and
[docs/research/GOOGLE_PLAY_TESTER_COMMUNITIES.md](../docs/research/GOOGLE_PLAY_TESTER_COMMUNITIES.md)
(tester-swap venues and what Google actually refuses).

## What you are actually asking for

Not "click this link". You are asking a stranger for **two weeks of keeping an
unknown app installed**, on **Android 8+**, signed in with **a Google Account**,
after a **~127 MB download**. Almost everything that reduces that ask works:
say how long it takes, say what happens to their phone, say that it is a paid
app but that testers do not pay — the $0 sale window, or a code from you — that
it has no ads and no account, and say what you will do with their feedback.

Two numbers to recruit against:

- **12** is the minimum at the moment you apply. The 14 days are counted back
  from the application, so the clock effectively starts when the twelfth tester
  opts in.
- **Recruit 15–18.** One tester who opts out on day 9 does not just subtract
  one; it moves the earliest possible application date, because the requirement
  is 12 *continuously* opted in.

Ask for the install, not for enthusiasm. A tester who says "sounds cool" and
never installs is worth nothing to the Console and nothing to you.

## Framing that works, and what not to do

- **Lead with the app, not with Google's rule.** "I built a handwriting grader
  for Chinese characters" gets read; "I need 12 testers or Google won't let me
  publish" gets scrolled past. Put the constraint second, briefly.
- **Filter for Android in the title.** These communities are full of iOS users,
  and Pleco users in particular often carry both. "Android testers" saves a
  dozen useless replies.
- **Disclose that you are the developer.** Every one of these communities
  removes undisclosed promotion, and some ban the disclosed kind too.
- **Ask a moderator before posting in a rules-bound community.** One modmail
  message costs two minutes and is the difference between a permanent post and
  a removal plus a reputation.
- **Ask for a specific kind of feedback** — "first ten minutes", "which
  characters it graded wrong", "what you would want next". Vague "any feedback
  welcome" gets none.
- **Do not pay testers or use tester-swap farms as your main plan.** Google
  asks how you recruited testers and whether their usage matched expected
  production behaviour; a cohort that installs a Chinese handwriting app and
  never opens it is what an "insufficient testing" rejection is made of. As a
  top-up to reach 12, it is a judgement call; as the plan, it is the plan that
  gets refused. Google's own recruitment guidance points at personal networks
  and communities where your future users already are — which is the rest of
  this file.
- **Never ask for Play reviews.** Test builds cannot be publicly reviewed, and
  asking is a policy problem.
- **A new Reddit account posting a link is removed automatically.** If you have
  no post history, either build some genuine history first or skip Reddit for
  the first push and use the forums and local routes below.

### Three things that are not negotiable

1. **Every tester installs from Play through the opt-in link.** Not an APK you
   send them, not a build from the repository. A documented case had 20 testers
   opted in for 14 days and was still refused because they had sideloaded: Play
   had no record that anyone was testing. See
   `docs/research/GOOGLE_PLAY_TESTER_COMMUNITIES.md`.
2. **The count is 12 *continuously* opted in**, measured back from the day you
   apply. Hence the over-recruitment.
3. **Engagement has to be visible — and you are the one who records it.** Play's
   statistics pages generally do not report test-track data, so the evidence is
   the Testing feedback page plus your own log. Ask testers for one dated task a
   week, and write down what they sent and what you changed. Check **Monitor and
   improve → Android vitals** anyway: crashes and ANRs from testers do show up,
   and a crash you fix mid-test is exactly the "we acted on feedback" material
   the application asks for.

## Where to ask

Ordered by expected value for *this* app — a serious, handwriting-and-HSK tool
for adult learners.

### Tier 1: the one place that explicitly invites this

[**Chinese-Forums → "Introduce Your Product Here"**](https://www.chinese-forums.com/forums/forum/73-introduce-your-product-here/)
is the only community found with a **verified, sanctioned, actively-used slot
for exactly this post**. The admin's product-promotion policy (July 2026) makes
it the only permitted place for product posts, says the one-post-per-product rule
still applies, and states that **free and open-source products are included**.

Post **once**, framed as a resource and feedback request, not an ad. Expect the
first post to pass through a moderation queue. Do not mention the app in other
boards, do not ask leading questions about it elsewhere, and do not bump it —
the policy says tiptoeing around it "may result in a ban". Recent precedent
includes another developer's "Help me test my sentence/grammar pattern app"
(September 2026) and Pleco's own beta sign-up thread, so this is a road others
have used. Full quotes in
[CHINESE_LEARNER_COMMUNITIES.md](../docs/research/CHINESE_LEARNER_COMMUNITIES.md).

### Tier 2: Discord, the largest audiences

| Server | Invite | Members |
| --- | --- | --- |
| 中英交流 Chinese-English Language Exchange | <https://discord.gg/TJT6KAxp4U> | 81,648 |
| r/ChineseLanguage's Discord | <https://discord.gg/G4nagat> | 44,762 |
| Yomitan (dictionary and Anki power users) | <https://discord.gg/YkQrXW6TXF> | 1,773 |
| Chinese (HSK tests, teachers, voice chat) | <https://discord.gg/Zvt4WWPsQN> | 564 |

Every one of these keeps its rules behind the join, so **nobody can tell you in
advance whether a recruitment post is welcome.** Ask a moderator in-server first,
and look for a #resources or #feedback channel rather than dropping it in general
chat. The upside: these servers skew US/EU/SEA, and Discord is blocked in
mainland China, so the audience is almost entirely outside the mainland — the
lowest access-risk group for a Google Play beta.

DISBOARD's own `chinese-learning` and `hsk` tags are not worth the effort: three
servers of 27–56 members, and two respectively.

### Tier 3: Reddit, only through a moderator

| Subreddit | Subscribers | What to do |
| --- | --- | --- |
| [r/ChineseLanguage](https://www.reddit.com/r/ChineseLanguage/) | 242,336 | Do not cold-post. Modmail to ask whether recruitment is allowed or belongs in a megathread, and ask about a listing on the curated `/wiki/Software` page. Self-promotion here is a long-running, heavily moderated sore point, and the mods have voided their own poll over it. |
| [r/Chinese_handwriting](https://www.reddit.com/r/Chinese_handwriting/) | 4,374 | Active and exactly on topic, but small and its rules could not be read — ask the mods. |
| [r/TestMyApp](https://www.reddit.com/r/TestMyApp/) | 4,557 | The one sub *designed* for developer testing. It requires the title format "**[v0.5, Android]** …" and says "No advertising apps. This is for development only"; non-conforming posts are deleted. |
| [r/LearnChinese](https://www.reddit.com/r/LearnChinese/) | 11,439 | **Restricted** — only approved users can submit. Go to its Discord (the 81k server above) or ask for approved-submitter status. |

Skip these: **r/languagelearning** bans "posts focused on one language" and warns
that owners of submitted content "may be banned without warning" (and a friend
posting it counts as you posting it); **r/LearnMandarin** says "No ads for your
own stuff!"; **r/Anki** is off topic for a non-Anki app; and
r/ChineseLanguageLearning, r/Chinesetutor, r/AnkiChinese, r/HSK and
r/AndroidTesters **do not exist**, however often lists of this rule's advice
mention them.

### Also worth an email, not a post

- **Hacking Chinese** (<https://www.hackingchinese.com/about/#contact>) is a
  blog, podcast and course site, not a verified community. Pitch Olle Linge the
  app as a free resource for his readers and ask whether he would mention it.
- **Pleco Forums** is the vendor's own very active support forum (7,992 members,
  separate Android and iOS sections) and hosts Pleco's own beta. There is no
  explicit rule against a competitor, but a competing Chinese app would plainly
  read as competitive advertising. Treat it as "ask `mikelove` for permission",
  not a place to post. He is also the Chinese-Forums admin, so one conversation
  covers both.
- **Facebook groups.** Only one could be confirmed to exist —
  [I'm Learning Mandarin](https://facebook.com/groups/imlearningmandarin) — and
  no group's size, activity or rules are verifiable without a logged-in account.
  Before posting to any group, read its rules tab; assume "admin approval
  required" until you have.
- **HSK-exam groups** are the same story: the audience is right, the rules are
  invisible from outside. Read the rules tab, then post once per group with the
  disclosure, the Android requirement and the five steps.

Two dead ends, so you do not rediscover them:
**how-to-learn-any-language.com no longer resolves at all**, and
**forum.language-learners.org** answered every automated request with a Cloudflare
human-verification page, so its size, activity and promotion rules are unknown —
somebody has to look in a browser before it counts as a venue.

### Fast, but the wrong kind of tester

Communities exist purely to swap closed-test slots, and services sell you twelve
testers. They fill the counter reliably and *pass the review* noticeably less
reliably, because they optimise opt-in rather than engagement. Details and
sources are in `docs/research/GOOGLE_PLAY_TESTER_COMMUNITIES.md`.

| Venue | Size | What it is |
| --- | --- | --- |
| [r/AndroidClosedTesting](https://www.reddit.com/r/AndroidClosedTesting/) | ~41k | Purpose-built to swap Play testing links. |
| [r/AndroidAppTesters](https://www.reddit.com/r/AndroidAppTesters/) | ~11k | The most on-topic of them; flairs include "12 testers needed". |
| r/20AndroidTesters, r/alphaandbetausers, r/betatests | 2k–46k | General beta recruitment; r/20AndroidTesters funnels to a paid service. |
| Discord: [Test My App](https://discord.gg/9wGENxCZrN) | ~690 | Linked from r/TestMyApp. [TestCrew](https://discord.gg/SF8JBExGq5) (~106) is a Japanese credit-based mutual-testing server. |
| Paid: TesterBee, Testers Community, PrimeTestLab, TestLaunch Pro | — | Around $15–$30 for 12–25 testers, per app. Approval-rate claims are unaudited marketing. |

Two naming traps: **r/playtesters is video-game and tabletop playtesting**, not
Android, and **r/AndroidTesters does not appear to exist** — both are repeated in
guides about this rule.

Note the documented failure that fits this app exactly: a developer with
**20 testers opted in for 14 days was still refused** because the testers had
installed a **sideloaded APK** rather than the Play version, so Play had no
record of them testing. Another developer in the same thread was refused with
"half of them through reddit forum". Google's stated reasons include
"insufficient tester engagement" and testers not using all available features.
So: swap groups as a top-up, real learners as the backbone, and Play installs
for everyone.

### The underrated route: people you can actually reach

Twelve strangers on the internet is hard. Twelve people from one class is easy,
and they are the app's real users:

- **University Chinese programs and Chinese student societies** — one email to
  a language department or society that lands gets you past twelve in a day.
- **Local Mandarin Meetup / conversation groups** — everyone there is already
  studying, and they are local to your timezone (you are at +1000).
- **Chinese teachers** — they test stroke grading faster and more usefully than
  anyone, and they know dozens of learners.
- **Your own network**, filtered for "does this person study Chinese", not
  "does this person own an Android phone".

### Channels you own, which should exist before you post anywhere

1. **A beta page on hherb.com.** The site is live and `dist/hanzi-tutor/`
   already exists, so `dist/hanzi-tutor/beta.html` is a small addition: the
   opt-in link, the five steps, the Android and two-week requirements, the
   feedback address, and a note that the app sells for A$14.99 but that testers
   never pay it — during the sale it installs free, and after the sale you send
   a code. Every post then links to one URL that explains everything, and the
   instructions survive the post being removed.
2. **The repository** (`github.com/hherb/hanzitutor`) — a short "want to test
   it?" section in the README, or a pinned issue. It recruits developers rather
   than HSK learners, but it costs nothing and it is where the privacy claims
   are already documented.

## Post templates

Adjust the specifics; the shape is what matters.

**Forum or subreddit post**

> **Offline HSK handwriting app (A$14.99 at launch, free to testers) — looking
> for Android testers (I'm the developer)**
>
> I built Hanzi Tutor, an Android app for learning to write simplified Chinese
> by hand. You write a character and it grades every stroke — shape, placement,
> ink and order — then tells you *why*: "not yet legible", "stroke order off",
> "wrong place". It teaches a frequency-ordered 7,744-character course, carries
> the full HSK 3.0 word list, colours every character by tone, and scores your
> tone from your own voice. Everything works offline; there is no account, no
> ads and no analytics.
>
> It is in closed testing on Google Play, which for a new personal developer
> account means running two weeks with at least twelve testers before it can be
> published. So I am looking for twelve people who would install it and leave it
> on their phone for two weeks.
>
> What it needs from you: an Android phone (8.0 or newer), a Google Account, a
> ~127 MB download, and keeping it installed for two weeks.
>
> It will sell for A$14.99, but nobody testing it pays: while the test runs it
> installs free. If Play ever shows you that price instead of **Install**, tell
> me and I will send you a code — please don't buy it.
>
> What you do:
> 1. Check the Play Store is signed in with the Google Account you want to use
>    (profile picture, top right).
> 2. Open this link on the phone: <opt-in link>
> 3. Press **Become a tester**, then **Download it on Google Play**, then
>    **Install**.
> 4. Use it a few times over the next two weeks.
>
> If you try it, the most useful thing you can send me is where the grading
> disagreed with you, and what you would want the app to do next. I am the
> developer and I will reply to everything.

**Chinese-Forums, "Introduce Your Product Here"** — the same content, but that
subforum's policy expects a substantive product introduction rather than a
one-line link drop, it must be primarily about Chinese, and it is the *only*
place your product may be mentioned. Add a line about what the grading actually
does with stroke geometry and where it disagrees with a teacher, keep the
screenshots and the "I'm the developer" disclosure, and make the tester request
the last paragraph rather than the headline.

**Discord or group chat**

> Hi — is there a channel here for sharing projects? I've built an Android app
> for practising Chinese handwriting (grades every stroke, HSK 3.0 word list,
> works offline, no account). It's in Google Play closed testing and I need
> Android testers to install it and keep it for two weeks. It sells for A$14.99
> but testers install it free — and if you ever see a price, I send a code.
> Happy to post details wherever you'd prefer. (I'm the developer.)

**Direct message to someone who studies Chinese**

> Hi — I saw your post about practising character writing. I've built an Android
> app that grades handwriting stroke by stroke against the HSK 3.0 list; it's
> offline, has no account, and sells for A$14.99 — but testers don't pay for it.
> It's in closed testing and I need Android testers who'll keep it installed for
> two weeks — would you want to try it? No problem at all if not.

## Tracking it

Keep this table somewhere you will update — the application form asks you to
summarise the test, and the dates are the difference between applying on day 15
and applying on day 21.

| Tester (Google Account) | Where they came from | Opted in (date) | Installed | Day 14 | Notes / feedback |
| --- | --- | --- | --- | --- | --- |
| my.list.subscriptions@gmail.com | own list | | | | |

The Console's **Monitor and improve → Ratings and reviews → Testing feedback**
is the record that the test was real; keep the emails too. Play's statistics
pages do not report test-track sessions, so this log is the only account of what
your testers actually did — and the production-access form asks you to
summarise exactly that.

A workable split, if the niche communities do not yield twelve on their own:
**8–10 genuine Chinese learners who use the app seriously**, topped up to 15–18
from r/AndroidClosedTesting or r/AndroidAppTesters. The genuine users give you
the engagement story and the quotable feedback; the top-up protects the
continuity count. Keep the split honest in your own notes, and never manufacture
extra accounts of your own — treat your own Google Account as at most one of the
twelve, and do not rely on it.

## Sequence

1. Add the beta page to hherb.com and settle the opt-in link.
2. Fix the Console so the list is attached and the link works (`tester-invite.md`).
3. **Settle the money before emailing anyone.** The app is priced at A$14.99 and
   closed-test testers are asked to pay it, so nothing else works until the $0
   sale and the promo codes are ready and verified: refund the testers who
   already paid, prove a code on a test account, and start the sale only once
   the twelve are lined up, because a sale cannot run longer than 14 days
   (`tester-invite.md`, "The app is priced"). Then email the two testers you
   already have — they are free progress.
4. **Post once to Chinese-Forums' "Introduce Your Product Here"** — the only
   venue that has explicitly said this kind of post belongs. Frame it as a free
   resource and a request for feedback.
5. **Modmail the r/ChineseLanguage moderators and ask a moderator in the two big
   Discords** before posting anywhere else. Ask, do not post and hope.
6. Work the local routes in parallel — a university Chinese class or a Meetup
   group can supply the whole twelve in one afternoon, and they are real users.
7. Over-recruit toward 15–18, then note the date the twelfth opts in.
8. Keep the test running while you act on feedback; the application asks what you
   changed because of it.
