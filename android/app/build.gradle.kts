plugins {
    id("com.android.application")
}

android {
    namespace = "com.duo.gmk3"
    compileSdk = 36

    defaultConfig {
        applicationId = "com.duo.gmk3"
        minSdk = 24              // 与 Rust 侧构建平台（NDK r30, platform 24）一致
        targetSdk = 36
        versionCode = 1
        versionName = "0.1.0"
    }

    // 固定共享签名密钥（提交在仓库里）：本地 dev.sh 与 GitHub CI 用同一把
    // 密钥签名，两边打出的 APK 可以互相覆盖安装、升级。
    // 这是个人分发用密钥（非 Play 上架）；以后要上架 Play 再换成私有密钥。
    signingConfigs {
        create("gmk3") {
            storeFile = rootProject.file("keystore/gmk3.keystore")
            storePassword = "gmk3gmk3"
            keyAlias = "gmk3"
            keyPassword = "gmk3gmk3"
        }
    }

    buildTypes {
        debug {
            signingConfig = signingConfigs.getByName("gmk3")
        }
        release {
            signingConfig = signingConfigs.getByName("gmk3")
            isMinifyEnabled = false
        }
    }

    // 纯原生工程没有 Java/Kotlin 源码，声明一下兼容级别即可
    compileOptions {
        sourceCompatibility = JavaVersion.VERSION_17
        targetCompatibility = JavaVersion.VERSION_17
    }

    // ABI 拆分：仅 CI 用（-PenableAbiSplits）一次产出三个 APK：
    //   app-arm64-v8a-release.apk / app-x86_64-release.apk / app-universal-release.apk
    // 本地 dev.sh 不传该属性，仍产出单一 app-release.apk，流程不变
    splits {
        abi {
            isEnable = project.hasProperty("enableAbiSplits")
            reset()
            include("arm64-v8a", "x86_64")
            isUniversalApk = true
        }
    }
}
