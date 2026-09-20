package com.hanzitutor.app

import android.Manifest
import android.content.pm.PackageManager
import android.os.Bundle
import android.webkit.WebView
import androidx.activity.OnBackPressedCallback
import androidx.activity.enableEdgeToEdge
import androidx.core.content.ContextCompat

/**
 * The Android entry point.
 *
 * Two things are Android's own rather than Tauri's, and both live here because
 * this is the only class the platform hands control to:
 *
 *  * **The back button.** `TauriActivity` sets `handleBackNavigation = false`, so
 *    nothing else is listening and the platform default — leave the app — would
 *    apply. What a learner expects first is for back to close the navigation
 *    sheet, exactly as Escape does at a keyboard, and only then to leave. So the
 *    webview is asked whether it consumed the press, and only a "no" finishes the
 *    activity.
 *  * **The microphone.** Tone practice records the learner's own voice. Android
 *    grants that at runtime, and an app that declares the permission without ever
 *    asking for it does not get silence it can detect — `cpal` opens a device
 *    happily and the samples are simply empty — so it is asked for once, on the
 *    first launch, before the learner has a reason to press the button.
 */
class MainActivity : TauriActivity() {
  /** The page's answer to "did you use that back press?". */
  private var webView: WebView? = null

  /**
   * Consumes the back press, and finishes the activity only if the page did not.
   *
   * Enabled from the start: `TauriActivity` sets `handleBackNavigation = false`,
   * so this is the only callback registered and there is nothing to defer to.
   * A press that arrives before the webview exists has no page to ask, and
   * leaving the app is the right answer then.
   */
  private val backPress = object : OnBackPressedCallback(true) {
    override fun handleOnBackPressed() {
      val view = webView
      if (view == null) {
        leave()
        return
      }
      // The callback comes back on this thread, so `leave()` is safe in it.
      view.evaluateJavascript(BACK_PROBE) { answer ->
        // `evaluateJavascript` hands back JSON, so a consumed press arrives as
        // the four characters `true`.
        if (answer != "true") {
          leave()
        }
      }
    }

    /** Let the platform have the press, which finishes the activity. */
    private fun leave() {
      isEnabled = false
      onBackPressedDispatcher.onBackPressed()
      isEnabled = true
    }
  }

  override fun onCreate(savedInstanceState: Bundle?) {
    enableEdgeToEdge()
    super.onCreate(savedInstanceState)
    onBackPressedDispatcher.addCallback(this, backPress)
    askForTheMicrophoneOnce()
  }

  override fun onWebViewCreate(webView: WebView) {
    this.webView = webView
    super.onWebViewCreate(webView)
  }

  /**
   * Tell the page that the permission question has been answered.
   *
   * Nothing else can: the page asked whether a microphone was usable before the
   * learner had answered, was told no, and has no way to know that the answer
   * has changed. Without this the tone-practice button stays disabled until the
   * app is restarted.
   */
  override fun onRequestPermissionsResult(
    requestCode: Int,
    permissions: Array<out String>,
    grantResults: IntArray,
  ) {
    super.onRequestPermissionsResult(requestCode, permissions, grantResults)
    if (requestCode == MICROPHONE_REQUEST) {
      webView?.evaluateJavascript(PERMISSIONS_CHANGED, null)
    }
  }

  /**
   * Ask for the microphone, once.
   *
   * `savedInstanceState == null` is what makes it once per launch rather than
   * once per rotation, and Android's own "don't ask again" is what makes it once
   * per install after the learner has answered.
   */
  private fun askForTheMicrophoneOnce() {
    val granted = ContextCompat.checkSelfPermission(this, Manifest.permission.RECORD_AUDIO) ==
      PackageManager.PERMISSION_GRANTED
    if (!granted) {
      requestPermissions(arrayOf(Manifest.permission.RECORD_AUDIO), MICROPHONE_REQUEST)
    }
  }

  private companion object {
    const val MICROPHONE_REQUEST = 4711

    /**
     * Asks the page whether it used the back press.
     *
     * A global function rather than an event listener so that the answer is
     * synchronous: `OnBackPressedCallback` has to decide *now* whether to
     * consume the press, and a message to the page and back is not a round trip
     * that can be waited for on the main thread. Absent — an older page, or one
     * that failed to start — the answer is falsy and the app exits, which is the
     * behaviour that needs no cooperation.
     */
    const val BACK_PROBE =
      "(window.__hanziHandleBack && window.__hanziHandleBack()) === true"

    /** Tells the page to ask about the microphone again. See the override above. */
    const val PERMISSIONS_CHANGED =
      "(window.__hanziPermissionsChanged && window.__hanziPermissionsChanged()) === true"
  }
}
