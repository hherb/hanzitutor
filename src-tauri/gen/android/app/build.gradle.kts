import java.util.Properties

plugins {
    id("com.android.application")
    id("org.jetbrains.kotlin.android")
    id("rust")
}

val tauriProperties = Properties().apply {
    val propFile = file("tauri.properties")
    if (propFile.exists()) {
        propFile.inputStream().use { load(it) }
    }
}

// The release signing key, which is deliberately *not* in the repository.
//
// `key.properties` lives at the root of the Android project (not in `app/`,
// which is this module's directory) because that is the file the `.gitignore`
// there excludes, so the passwords and the keystore's location stay on the
// machine that builds the release. That is also why the file is optional:
// without it the release build is simply unsigned, which is the honest outcome
// for a clone that has no key. The keystore itself lives outside the checkout
// entirely (under `~/.android`), because a signing key that can be committed is
// a signing key that eventually will be.
val keystoreProperties = Properties().apply {
    val propFile = rootProject.file("key.properties")
    if (propFile.exists()) {
        propFile.inputStream().use { load(it) }
    }
}

android {
    compileSdk = 36
    // Which NDK to use, named explicitly because Gradle needs it for more than
    // compiling: `llvm-strip` lives in the NDK, and without a `ndkVersion` Gradle
    // cannot find it and quietly gives up on stripping the native library
    // ("Unable to strip the following libraries, packaging them as they are"),
    // which is a 212 MB debug library shipped verbatim. Taken from the same
    // environment variable the Rust side is built with, so one setting drives
    // both, with the version this project is developed against as the fallback
    // for a build started from Android Studio, where no such variable is set.
    ndkVersion = System.getenv("ANDROID_NDK_HOME")?.let { File(it).name } ?: "29.0.14206865"
    namespace = "com.hanzitutor.app"
    defaultConfig {
        manifestPlaceholders["usesCleartextTraffic"] = "false"
        applicationId = "com.hanzitutor.app"
        // 26, not the template's 24: tone practice captures through `cpal`,
        // whose Android backend drives AAudio, and cpal pins the `ndk` crate to
        // its `api-level-26` feature. AAudio's `libaaudio.so` does not exist
        // before Android 8, so at `minSdk = 24` the link fails outright with
        // `unable to find library -laaudio` and, had it linked, the produced
        // `.so` would carry a DT_NEEDED the loader cannot satisfy on an API
        // 24/25 device. This file is generated once by `android init`, so the
        // matching `bundle.android.minSdkVersion` in `tauri.conf.json` — which
        // is what the CLI uses to choose the NDK linker's API level — has to
        // agree with it. Android 8 (2017) is old enough that nothing is lost.
        minSdk = 26
        targetSdk = 36
        versionCode = tauriProperties.getProperty("tauri.android.versionCode", "1").toInt()
        versionName = tauriProperties.getProperty("tauri.android.versionName", "1.0")
    }
    signingConfigs {
        // Created only when there is a key to create it from, so a clone with no
        // `key.properties` still configures — it just cannot sign a release.
        if (keystoreProperties.getProperty("storeFile") != null) {
            create("release") {
                storeFile = file(keystoreProperties.getProperty("storeFile"))
                storePassword = keystoreProperties.getProperty("storePassword")
                keyAlias = keystoreProperties.getProperty("keyAlias")
                keyPassword = keystoreProperties.getProperty("keyPassword")
            }
        }
    }
    buildTypes {
        getByName("debug") {
            manifestPlaceholders["usesCleartextTraffic"] = "true"
            isDebuggable = true
            isJniDebuggable = true
            isMinifyEnabled = false
            // The template asks Gradle to keep the debug symbols in the packaged
            // `.so`, which is the right default for an app whose native code is
            // a little JNI glue. Here the native library *is* the application,
            // and a debug build of it is 212 MB — an APK that will not fit on a
            // stock emulator's data partition and takes minutes to install over
            // USB, on every change. Stripping it brings the APK down to tens of
            // megabytes. Nothing is lost that matters: panics are reported by
            // message through logcat, and the unstripped library is still in
            // `.cargo-target/` for `llvm-symbolizer` / `ndk-stack` when a native
            // backtrace really does need line numbers. Put these lines back if
            // native single-stepping in Android Studio is ever wanted.
        }
        getByName("release") {
            isMinifyEnabled = true
            proguardFiles(
                *fileTree(".") { include("**/*.pro") }
                    .plus(getDefaultProguardFile("proguard-android-optimize.txt"))
                    .toList().toTypedArray()
            )
            // Null when there is no `key.properties`: the bundle is then built
            // unsigned rather than signed with the debug key, so that an unsigned
            // artifact can never be mistaken for a shippable one. The reflective
            // paths R8 must not break are already covered by the consumer rules
            // Tauri's Android library ships — see HANDOVER §6.
            signingConfig = signingConfigs.findByName("release")
        }
    }
    kotlinOptions {
        jvmTarget = "1.8"
    }
    buildFeatures {
        buildConfig = true
    }
}

rust {
    rootDirRel = "../../../"
}

dependencies {
    implementation("androidx.webkit:webkit:1.14.0")
    implementation("androidx.appcompat:appcompat:1.7.1")
    implementation("androidx.activity:activity-ktx:1.10.1")
    implementation("com.google.android.material:material:1.12.0")
    implementation("androidx.lifecycle:lifecycle-process:2.10.0")
    testImplementation("junit:junit:4.13.2")
    androidTestImplementation("androidx.test.ext:junit:1.1.4")
    androidTestImplementation("androidx.test.espresso:espresso-core:3.5.0")
}

apply(from = "tauri.build.gradle.kts")