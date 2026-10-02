# Getting testers into the closed test

`com.hherb.hanzitutor` · closed testing track · personal developer account
`8700454726233630990`

## The short version

Google **does not invite testers for you and does not email them**. Adding an
address to a tester list only makes that account *eligible*. Nothing arrives in
anyone's inbox, and the Console's Dashboard keeps saying `0 testers currently
opted in` until each tester opens the opt-in link, presses **Become a tester**,
and installs the app from Play.

Testers do **not** register anywhere, do not need a Play Console account, and do
not need an APK. One Google Account plus one link is the whole requirement.

**The link** — copy the live one from Play Console rather than trusting this
line. For this app it is:

```
https://play.google.com/apps/testing/com.hherb.hanzitutor
```

That is the URL to hand out. `https://play.google.com/store/apps/details?id=com.hherb.hanzitutor`
also works **after** opt-in; before production the app is not searchable in Play.

## What you do in Play Console first

Play Console → **Test and release → Testing → Closed testing** → **Manage track**.

1. **Testers tab.** Under *Testers*, choose the access method **Email** (not
   Google Groups, unless your testers really are in a Google Group), tick your
   tester list, and click **Save changes**. A list that exists but is not
   ticked here does nothing — this is the most common cause of "I added them and
   nobody can get in".
2. Confirm the exact address `my.list.subscriptions@gmail.com` is in that list,
   spelled correctly. Play matches the address to the Google Account that is
   signed in to the Play Store app; a typo is indistinguishable from the tester
   not bothering.
3. **Copy the shareable link** shown on that same tab. Compare it with the URL
   above. If the Console shows no opt-in link at all, the release is not
   *Published* yet.
4. **Release status.** On the closed testing track's Releases tab the release
   must read **Available to testers** with a **100%** rollout. `In review` or
   `Pending publication` means testers get an error page. After a first-ever
   publish the link itself can take several hours to start working.
5. **Countries.** Check the track's country availability includes the testers'
   countries. Testers in an excluded country see *item not found*.
6. **Advanced settings.** Do not tick *Turn on* under Managed Google Play unless
   every tester is in a managed organisation — it makes the app private and
   unsearchable.

Then watch **Test and release → Testing → Closed testing** for the count, and
**Monitor and improve → Ratings and reviews → Testing feedback** for what they
send. The Dashboard number lags, often by hours.

## What the tester does

Send them the link and these five steps.

1. On the Android phone, open the Play Store app → tap the profile circle →
   confirm which Google Account is signed in. **It must be
   `my.list.subscriptions@gmail.com`.** If it is not, add that account
   (Settings → Google → Add account) and switch to it in the Play Store — the
   opt-in only attaches to the account on your list.
2. Open the opt-in link in a browser on that phone:
   `https://play.google.com/apps/testing/com.hherb.hanzitutor`
3. Press **Become a tester**. The page changes to a "You're a tester" style
   confirmation with a **Download it on Google Play** link.
4. Follow that link and press **Install** (about 127 MB, Android 8.0+). If it
   shows a price instead — A$14.99 — stop there and ask for a code rather than
   paying it. Do not sideload an APK — an app installed by hand does not count
   as opted in.
5. Open the app once, use it a little, and reply with feedback.
   **Then leave it alone in the good sense:** stay opted in, keep the app
   installed, and do not press *Leave the test* for at least 14 days.

## The email to send

> Subject: Please test Hanzi Tutor on Android (about 5 minutes to set up)
>
> Hi — could you help me test my Android app, Hanzi Tutor, before I release it?
> You don't need to register anywhere or install any special build; it comes
> from Google Play like any other app. Two things matter:
>
> 1. It must be the Google Account `my.list.subscriptions@gmail.com` that is
>    signed in to the Play Store on the phone. In the Play Store app, tap your
>    profile picture (top right) to check. If it's a different account, add
>    that one and switch to it.
> 2. Open this link on the phone:
>    https://play.google.com/apps/testing/com.hherb.hanzitutor
>    Press **Become a tester**, then **Download it on Google Play**, then
>    **Install** (about 127 MB; needs Android 8.0 or newer). The app sells for
>    A$14.99, but you should never be asked to pay: if a price appears instead
>    of **Install**, reply to this email and I will send you a code for it.
>
> Then just open it and use it for a few minutes. Please **stay opted in and
> keep the app installed for at least 14 days** — Google counts continuous
> opt-in, and leaving the test early means the day count starts again. Please
> don't uninstall or press "Leave the test" until I say the test is over.
>
> Anything that breaks, confuses you, or looks wrong is useful — reply to this
> email with it. Thanks!

## The rule you are actually up against

This is a personal developer account created after 13 November 2023, so
production access needs **at least 12 testers who have been opted in
continuously for the preceding 14 days** when you apply. See
`store/listing.md` and Google's
[app testing requirements](https://support.google.com/googleplay/android-developer/answer/14151465).

- **Two testers is not close.** The two on the list today need eleven more.
- The 14 days are counted back from the day you apply, so the clock effectively
  starts when the **twelfth** tester opts in. Note that date.
- Opting out, leaving the test, or switching back to the internal track resets
  that tester's continuity. Internal testing is inactive here, so that
  particular trap is closed.
- Counted testers must be distinct Google Accounts. Twelve accounts you own that
  no one ever opens is exactly what the production-access questionnaire asks
  about, and "insufficient tester engagement" is a stated reason Google makes
  you keep testing.
- **Play must be the installation route, for everyone.** A developer with 20
  testers opted in for the full 14 days was still refused because they had
  installed a sideloaded APK, leaving Play no record of them testing — and a
  second developer in that same thread was refused with "half of them through
  reddit forum". Opting in and installing from Play are not paperwork; they are
  the evidence. Details in
  `docs/research/GOOGLE_PLAY_TESTER_COMMUNITIES.md`.
- The organisation account (D-U-N-S pending) is exempt from all of this; if it
  lands before you have twelve real testers, that is the shorter path.

For recruiting the other eleven, see `store/beta-recruiting.md`.

## When it does not work

| Symptom | Cause |
| --- | --- |
| Nothing arrives by email | Expected. Google sends nothing; you send the link. |
| *Item not found* / *We're sorry, the requested URL was not found* | Release not Published, link not yet propagated after first publish, or the app is not in the tester's country. |
| Play shows **A$14.99** instead of **Install** | Expected. The listing is paid and testers must *buy* a paid app in a closed test; what is wrong is that the $0 sale is not live or that tester has no promo code. See "The app is priced" below. If the page is not `com.hherb.hanzitutor`, they are on another developer's app. |
| Opt-in page loads, **Become a tester** errors | The signed-in Google Account is not on the tester list (wrong account, or a typo in the list), or the list is not ticked on the track. |
| Opted in, Play shows *Install* but it fails | Android below 8.0, an unsupported device, or the App Signing/ABI setup; check with `scripts/check-android-release.sh`. |
| Tester was in an older internal test | They must leave the internal test, then opt into closed testing. |
| Dashboard still says 0 after everyone opted in | Counts lag; check *Testing feedback* for real activity and wait. |
| Everything looks right and they still cannot join | Clear Play Store cache and reopen the opt-in link; if it persists, go back to *Manage track → Testers* and re-save the list so the track re-syncs. |

## The app is priced, so testers have to be let in for nothing

`com.hherb.hanzitutor` is set to **AUD 14.99** in **Monetize with Play →
Products → App pricing**. That is deliberate and it stays. Google's rule then
applies in full, and it is the opposite of the intuition that testers are
exempt:

> **Paid apps:** Testers must purchase paid apps when participating in open or
> closed tests. For internal tests, testers can install paid apps for free.

— [Set up an open, closed, or internal test](https://support.google.com/googleplay/android-developer/answer/9845334).
Internal testing is not a way round this: it is the one track that hands a paid
app over for nothing, and a user who opts into it is *barred* from the closed
test until they opt out, so they stop counting.

The twelve are therefore let in by two mechanisms, used together: a **$0 sale**
over the recruiting push, and **one-time promo codes** for anyone who arrives
after the sale ends.

**Do not "fix" this by setting the app free.** Paid → free is allowed; free →
paid never is, and the price could never come back under this package name
([Set up your app's prices](https://support.google.com/googleplay/android-developer/answer/6334373)).

### Set up, in this order, before the first tester opts in

1. **Refund the testers who already paid.** Anyone who installed while the price
   was live was charged A$14.99. **Play Console → Order management** issues the
   refund, and it is cheaper than losing the referrals.
2. **Create the sale.** App pricing → **Sales** → **Create sale**, discount type
   **Fixed price**, value **0**. A $0 sale sets the price to zero in every region
   automatically, is temporary, and "does not impact your ability to charge for
   the app in the future".
3. **Create the promotion.** **Monetize with Play → Promo codes** → **Create
   promo code**, type **one-time use** — the multi-use custom codes are
   subscriptions only — status **On**, and enough codes for the stragglers. The
   CSV downloads a few seconds later; each person gets their own.
4. **Verify both mechanisms on test accounts.** On a second account that is
   already on the tester list, opt in through the link and confirm the store
   page says **Install** while the sale is live, and watch that the app still
   opens after the sale ends. Then redeem one of the codes on a third account.
   Two things here are undocumented and have to be seen rather than assumed:
   whether a promo code redeems at all for an app whose only live track is
   closed testing, and whether an install acquired during a $0 sale still counts
   as owned afterwards — $0 sales "don't count as purchases", so they do not
   appear in the finance reports at all.

### The calendar, which is the part that bites

- A sale runs **1–14 days**, and **30 days** must pass between one sale ending
  and the next starting ([Create sales for paid apps](https://support.google.com/googleplay/android-developer/answer/7271135)).
  The 14-day cap is exactly the length of the test, so there is **no margin**: a
  slip cannot be absorbed by extending the sale, and the next one is a month
  away. Start the sale when the twelve are lined up, not while recruiting is
  still cold.
- The sale must be live **before** testers opt in, because it is the store page
  they land on that has to say **Install**.
- What Google counts is **opt-in continuity**, which the price does not touch
  once someone is in: a tester who installs during the sale goes on counting
  after it ends.
- Anyone who opts in after the sale ends needs a **promo code**, and in that
  order: opt in first, then redeem. Redeeming a code is a purchase action, not an
  opt-in, and neither substitutes for the other.
- Both mechanisms need the app *published*, and pricing is not per-track:
  "changes to your app's pricing affect all versions across all tracks".

## After the 14 days

When the Console shows 12+ testers opted in continuously and the days are up:
Dashboard → **Apply for production**, which asks for three things worth keeping
notes on now — how hard testers were to recruit, whether they used all the
app's features, and what you changed because of their feedback. Keep the
feedback emails and the Closed Test bug log; the application asks you to
summarise them.
