use std::env;
use std::path::PathBuf;

fn main() {
    println!("cargo:rerun-if-changed=libarchive/");

    // Build libarchive using CMake
    build_libarchive();

    // Generate Rust bindings
    generate_bindings();
}

fn build_libarchive() {
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
    let target = env::var("TARGET").unwrap();

    // Build libarchive with CMake, targeting only the static library
    let mut config = cmake::Config::new("libarchive");
    config
        // Build as static library
        .define("BUILD_SHARED_LIBS", "OFF")
        // Disable building tests, examples, and tools
        .define("ENABLE_TEST", "OFF")
        .define("ENABLE_TAR", "OFF")
        .define("ENABLE_CPIO", "OFF")
        .define("ENABLE_CAT", "OFF")
        .define("ENABLE_UNZIP", "OFF")
        // Disable optional features that might cause issues
        .define("ENABLE_WERROR", "OFF");

    // Android-specific CMake configuration
    if target.contains("android") {
        // Get Android NDK path
        let ndk_home = env::var("ANDROID_NDK_HOME")
            .or_else(|_| env::var("NDK_HOME"))
            .or_else(|_| env::var("ANDROID_NDK"))
            .expect("ANDROID_NDK_HOME environment variable must be set for Android builds");

        let ndk_path = PathBuf::from(&ndk_home);

        // Determine Android ABI and API level
        let android_abi = if target.contains("aarch64") {
            "arm64-v8a"
        } else if target.contains("armv7") {
            "armeabi-v7a"
        } else if target.contains("i686") {
            "x86"
        } else if target.contains("x86_64") {
            "x86_64"
        } else {
            "arm64-v8a"
        };

        // Use Android NDK toolchain file
        let toolchain_file = ndk_path.join("build/cmake/android.toolchain.cmake");

        if toolchain_file.exists() {
            // Set environment variables to point to NDK compilers to avoid "tool not found" warnings
            let ndk_bin = ndk_path.join("toolchains/llvm/prebuilt/darwin-x86_64/bin");
            let arch_prefix = if target.contains("armv7") {
                "armv7a-linux-androideabi"
            } else if target.contains("aarch64") {
                "aarch64-linux-android"
            } else if target.contains("i686") {
                "i686-linux-android"
            } else if target.contains("x86_64") {
                "x86_64-linux-android"
            } else {
                "aarch64-linux-android"
            };

            let c_compiler = ndk_bin.join(format!("{}21-clang", arch_prefix));
            let cpp_compiler = ndk_bin.join(format!("{}21-clang++", arch_prefix));

            // Set environment variables that cc crate respects to avoid detection warnings
            if c_compiler.exists() {
                // SAFETY: Setting environment variables during build script execution is safe
                // as we're in single-threaded build script context
                unsafe {
                    env::set_var(format!("CC_{}", target.replace("-", "_")), &c_compiler);
                }
            }
            if cpp_compiler.exists() {
                // SAFETY: Setting environment variables during build script execution is safe
                // as we're in single-threaded build script context
                unsafe {
                    env::set_var(format!("CXX_{}", target.replace("-", "_")), &cpp_compiler);
                }
            }

            // Use init_c_cfg() to prevent the cmake crate from detecting the host OS
            config.init_c_cfg(cc::Build::new());

            config.define("CMAKE_TOOLCHAIN_FILE", toolchain_file.to_str().unwrap());
            config.define("ANDROID_ABI", android_abi);
            config.define("ANDROID_PLATFORM", "android-21"); // Minimum API 21
            config.define("ANDROID_NDK", &ndk_home);
            config.define("CMAKE_SYSTEM_NAME", "Android");
        }

        // Android - enable all features like other platforms
        config
            .define("ENABLE_ACL", "ON")
            .define("ENABLE_XATTR", "ON")
            .define("ENABLE_ZLIB", "ON")
            .define("ENABLE_BZip2", "ON")
            .define("ENABLE_LZMA", "ON")
            .define("ENABLE_ZSTD", "ON")
            .define("ENABLE_LZ4", "ON");
    } else if target.contains("linux") && !cfg!(target_os = "linux") {
        // Cross-compiling to Linux from another platform

        // Set environment variables to point to Linux cross-compiler to avoid "tool not found" warnings
        let linux_gcc_names = vec!["x86_64-unknown-linux-gnu-gcc", "x86_64-linux-gnu-gcc"];

        for gcc_name in &linux_gcc_names {
            if let Ok(gcc_path) = which::which(gcc_name) {
                let gxx_name = gcc_name.replace("gcc", "g++");
                if let Ok(gxx_path) = which::which(&gxx_name) {
                    // SAFETY: Setting environment variables during build script execution is safe
                    // as we're in single-threaded build script context
                    unsafe {
                        env::set_var(format!("CC_{}", target.replace("-", "_")), &gcc_path);
                        env::set_var(format!("CXX_{}", target.replace("-", "_")), &gxx_path);
                    }
                    break;
                }
            }
        }

        // Use init_c_cfg() to prevent the cmake crate from using wrong compiler names
        config.init_c_cfg(cc::Build::new());

        // Full features for Linux
        config
            .define("ENABLE_ACL", "ON")
            .define("ENABLE_XATTR", "ON")
            .define("ENABLE_ZLIB", "ON")
            .define("ENABLE_BZip2", "ON")
            .define("ENABLE_LZMA", "ON")
            .define("ENABLE_ZSTD", "ON")
            .define("ENABLE_LZ4", "ON");
    } else if target.contains("windows") && !cfg!(target_os = "windows") {
        // Cross-compiling to Windows from another platform

        // Set environment variables to point to MinGW cross-compiler to avoid "tool not found" warnings
        if let Ok(mingw_gcc) = which::which("x86_64-w64-mingw32-gcc") {
            if let Ok(mingw_gxx) = which::which("x86_64-w64-mingw32-g++") {
                // SAFETY: Setting environment variables during build script execution is safe
                // as we're in single-threaded build script context
                unsafe {
                    env::set_var(format!("CC_{}", target.replace("-", "_")), &mingw_gcc);
                    env::set_var(format!("CXX_{}", target.replace("-", "_")), &mingw_gxx);
                }
            }
        }

        // Use init_c_cfg() to prevent the cmake crate from auto-detecting wrong platform
        config.init_c_cfg(cc::Build::new());

        // Full features for Windows
        config
            .define("ENABLE_ACL", "ON")
            .define("ENABLE_XATTR", "ON")
            .define("ENABLE_ZLIB", "ON")
            .define("ENABLE_BZip2", "ON")
            .define("ENABLE_LZMA", "ON")
            .define("ENABLE_ZSTD", "ON")
            .define("ENABLE_LZ4", "ON");
    } else if target.contains("wasm") {
        // WASM is not fully supported by libarchive v3.8.1
        // The library requires POSIX types (pid_t, uid_t, gid_t, etc.) that don't exist in WASM
        // CMake configuration fails when trying to detect these types
        panic!(
            "WASM target is not supported by libarchive v3.8.1. \
                The library requires POSIX types and system calls that are not available in WebAssembly. \
                Supported platforms: macOS, Windows, Linux, iOS, Android"
        );
    } else if target.contains("windows") && cfg!(target_os = "windows") {
        // Native Windows build (not cross-compiling)
        // Use vcpkg toolchain file if available
        if let Ok(vcpkg_root) = env::var("VCPKG_INSTALLATION_ROOT") {
            let toolchain_file = PathBuf::from(&vcpkg_root)
                .join("scripts/buildsystems/vcpkg.cmake");
            if toolchain_file.exists() {
                config.define("CMAKE_TOOLCHAIN_FILE", toolchain_file.to_str().unwrap());
            }
        }

        // Full features for Windows
        config
            .define("ENABLE_ACL", "ON")
            .define("ENABLE_XATTR", "ON")
            .define("ENABLE_ZLIB", "ON")
            .define("ENABLE_BZip2", "ON")
            .define("ENABLE_LZMA", "ON")
            .define("ENABLE_ZSTD", "ON")
            .define("ENABLE_LZ4", "ON")
            .define("ENABLE_OPENSSL", "ON");
    } else {
        // Full features for native platforms (macOS native, iOS, etc.)
        config
            .define("ENABLE_ACL", "ON")
            .define("ENABLE_XATTR", "ON")
            .define("ENABLE_ZLIB", "ON")
            .define("ENABLE_BZip2", "ON")
            .define("ENABLE_LZMA", "ON")
            .define("ENABLE_ZSTD", "ON")
            .define("ENABLE_LZ4", "ON");
    }

    // Only build the archive_static target to avoid installation issues
    config.build_target("archive_static");

    let _ = config.build();

    // The cmake crate returns the installation directory, but we want the build directory
    // The library is in OUT_DIR/build/libarchive
    let build_dir = out_dir.join("build");

    // On Windows MSVC, the library might be in Debug or Release subdirectory
    if target.contains("windows") && target.contains("msvc") {
        // On Windows MSVC, CMake builds to configuration-specific directories
        // The library file is named "archive.lib" not "archive_static.lib"
        println!(
            "cargo:rustc-link-search=native={}/libarchive/Debug",
            build_dir.display()
        );
        println!(
            "cargo:rustc-link-search=native={}/libarchive/Release",
            build_dir.display()
        );
        println!(
            "cargo:rustc-link-search=native={}/libarchive",
            build_dir.display()
        );
        // The actual library name from CMake is "archive.lib"
        println!("cargo:rustc-link-lib=static=archive");
    } else {
        println!(
            "cargo:rustc-link-search=native={}/libarchive",
            build_dir.display()
        );
        println!("cargo:rustc-link-lib=static=archive");
    }

    // Link system libraries that libarchive depends on
    let target = env::var("TARGET").unwrap();

    if target.contains("apple-darwin") {
        // macOS
        println!("cargo:rustc-link-search=native=/opt/homebrew/lib");
        println!("cargo:rustc-link-search=native=/usr/local/lib");
        println!("cargo:rustc-link-lib=iconv");
        println!("cargo:rustc-link-lib=xml2");
        println!("cargo:rustc-link-lib=b2");
        println!("cargo:rustc-link-lib=framework=CoreFoundation");
        println!("cargo:rustc-link-lib=z");
        println!("cargo:rustc-link-lib=bz2");
        println!("cargo:rustc-link-lib=lzma");
        println!("cargo:rustc-link-lib=zstd");
        println!("cargo:rustc-link-lib=lz4");
    } else if target.contains("apple-ios") {
        // iOS
        println!("cargo:rustc-link-lib=iconv");
        println!("cargo:rustc-link-lib=xml2");
        println!("cargo:rustc-link-lib=framework=CoreFoundation");
        println!("cargo:rustc-link-lib=z");
        println!("cargo:rustc-link-lib=bz2");
        println!("cargo:rustc-link-lib=lzma");
        println!("cargo:rustc-link-lib=zstd");
        println!("cargo:rustc-link-lib=lz4");
    } else if target.contains("linux-android") {
        // Android - link all compression libraries
        println!("cargo:rustc-link-lib=z");
        println!("cargo:rustc-link-lib=bz2");
        println!("cargo:rustc-link-lib=lzma");
        println!("cargo:rustc-link-lib=zstd");
        println!("cargo:rustc-link-lib=lz4");
    } else if target.contains("linux") {
        // Linux
        println!("cargo:rustc-link-lib=pthread");
        println!("cargo:rustc-link-lib=z");
        println!("cargo:rustc-link-lib=bz2");
        println!("cargo:rustc-link-lib=lzma");
        println!("cargo:rustc-link-lib=zstd");
        println!("cargo:rustc-link-lib=lz4");
        // Additional libraries for Linux features
        println!("cargo:rustc-link-lib=xml2"); // For XAR format support
        println!("cargo:rustc-link-lib=crypto"); // For encryption support (OpenSSL)
        println!("cargo:rustc-link-lib=acl"); // For POSIX ACL support
    } else if target.contains("windows") {
        // Windows - library names are different
        if target.contains("msvc") {
            // MSVC toolchain
            // Add vcpkg library path if VCPKG_INSTALLATION_ROOT is set
            if let Ok(vcpkg_root) = env::var("VCPKG_INSTALLATION_ROOT") {
                let vcpkg_lib = PathBuf::from(vcpkg_root)
                    .join("installed")
                    .join("x64-windows")
                    .join("lib");
                if vcpkg_lib.exists() {
                    println!("cargo:rustc-link-search=native={}", vcpkg_lib.display());
                }
            }
            println!("cargo:rustc-link-lib=zlib");
            println!("cargo:rustc-link-lib=bz2");
            println!("cargo:rustc-link-lib=lzma");
            println!("cargo:rustc-link-lib=zstd");
            println!("cargo:rustc-link-lib=lz4");
            println!("cargo:rustc-link-lib=libcrypto"); // OpenSSL libcrypto for digest functions
            println!("cargo:rustc-link-lib=bcrypt"); // For additional crypto functions
            println!("cargo:rustc-link-lib=advapi32");
        } else {
            // MinGW toolchain
            println!("cargo:rustc-link-lib=z");
            println!("cargo:rustc-link-lib=bz2");
            println!("cargo:rustc-link-lib=lzma");
            println!("cargo:rustc-link-lib=zstd");
            println!("cargo:rustc-link-lib=lz4");
            println!("cargo:rustc-link-lib=crypto"); // OpenSSL libcrypto for digest functions
            println!("cargo:rustc-link-lib=bcrypt");
            println!("cargo:rustc-link-lib=advapi32");
        }
    } else if target.contains("wasm") {
        // WASM - only zlib is enabled, link via emscripten
        println!("cargo:rustc-link-lib=z");
    }
}

fn generate_bindings() {
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
    let libarchive_include = PathBuf::from("libarchive/libarchive");
    let target = env::var("TARGET").unwrap();

    // Generate bindings
    let mut builder = bindgen::Builder::default()
        .header("wrapper.h")
        .clang_arg(format!("-I{}", libarchive_include.display()))
        // Types to generate
        .allowlist_type("archive.*")
        .allowlist_type("archive_entry.*")
        // Functions to generate
        .allowlist_function("archive.*")
        // Variables to generate
        .allowlist_var("ARCHIVE.*")
        .allowlist_var("AE_.*")
        // Generate constants
        .parse_callbacks(Box::new(bindgen::CargoCallbacks::new()))
        // Formatting
        .derive_debug(true)
        .derive_default(true)
        .size_t_is_usize(true)
        // Disable layout tests to avoid cross-compilation struct size mismatches
        .layout_tests(false);

    // Add platform-specific clang arguments for cross-compilation
    if target.contains("windows") {
        builder = builder
            .clang_arg("--target=x86_64-pc-windows-gnu")
            .clang_arg(format!("-I{}/build", out_dir.display()));

        // Add MinGW sysroot if available
        if let Ok(mingw_gcc) = which::which("x86_64-w64-mingw32-gcc") {
            // Get the real path (resolve symlinks)
            if let Ok(real_path) = std::fs::canonicalize(&mingw_gcc) {
                // Path structure: /opt/homebrew/Cellar/mingw-w64/VERSION/toolchain-x86_64/bin/x86_64-w64-mingw32-gcc
                // We need: /opt/homebrew/Cellar/mingw-w64/VERSION/toolchain-x86_64/x86_64-w64-mingw32/include
                if let Some(bin_dir) = real_path.parent() {
                    if let Some(toolchain_dir) = bin_dir.parent() {
                        let mingw_include = toolchain_dir.join("x86_64-w64-mingw32/include");
                        if mingw_include.exists() {
                            builder = builder.clang_arg(format!("-I{}", mingw_include.display()));
                        }
                    }
                }
            }
        }
    } else if target.contains("android") {
        // Android cross-compilation
        builder = builder.clang_arg(format!("-I{}/build", out_dir.display()));

        // Determine Android architecture and API level
        let ndk_triple = if target.contains("aarch64") {
            "aarch64-linux-android"
        } else if target.contains("armv7") {
            "armv7a-linux-androideabi"
        } else if target.contains("i686") {
            "i686-linux-android"
        } else if target.contains("x86_64") {
            "x86_64-linux-android"
        } else {
            "aarch64-linux-android"
        };

        builder = builder.clang_arg(format!("--target={}", ndk_triple));

        // Try to find Android NDK sysroot
        let ndk_home = env::var("ANDROID_NDK_HOME")
            .or_else(|_| env::var("NDK_HOME"))
            .or_else(|_| env::var("ANDROID_NDK"))
            .ok();

        if let Some(ndk_home) = ndk_home {
            let ndk_path = PathBuf::from(ndk_home);

            // Modern NDK structure (r19+)
            let sysroot = ndk_path.join("toolchains/llvm/prebuilt/darwin-x86_64/sysroot");

            if sysroot.exists() {
                builder = builder
                    .clang_arg(format!("--sysroot={}", sysroot.display()))
                    .clang_arg(format!("-I{}/usr/include", sysroot.display()))
                    .clang_arg(format!(
                        "-I{}/usr/include/{}",
                        sysroot.display(),
                        ndk_triple
                    ));
            }
        }
    } else if target.contains("linux") && !cfg!(target_os = "linux") {
        // Cross-compiling to Linux from another platform
        builder = builder
            .clang_arg("--target=x86_64-unknown-linux-gnu")
            .clang_arg(format!("-I{}/build", out_dir.display()));

        // Try to find Linux cross-compiler sysroot
        let possible_gcc_names = vec!["x86_64-unknown-linux-gnu-gcc", "x86_64-linux-gnu-gcc"];

        for gcc_name in possible_gcc_names {
            if let Ok(linux_gcc) = which::which(gcc_name) {
                // Resolve symlinks to get real path
                // Symlink: /opt/homebrew/Cellar/x86_64-unknown-linux-gnu/VERSION/bin/x86_64-unknown-linux-gnu-gcc
                // Realpath: /opt/homebrew/Cellar/x86_64-unknown-linux-gnu/VERSION/toolchain/bin/x86_64-unknown-linux-gnu-gcc
                // We need: /opt/homebrew/Cellar/x86_64-unknown-linux-gnu/VERSION/toolchain/x86_64-unknown-linux-gnu/sysroot
                if let Ok(real_path) = std::fs::canonicalize(&linux_gcc) {
                    let sysroot = real_path
                        .parent()
                        .and_then(|p| p.parent())
                        .map(|p| p.join("x86_64-unknown-linux-gnu/sysroot"));

                    if let Some(sysroot) = sysroot {
                        if sysroot.exists() {
                            builder = builder
                                .clang_arg(format!("--sysroot={}", sysroot.display()))
                                .clang_arg(format!("-I{}/usr/include", sysroot.display()));
                            break;
                        }
                    }
                }
            }
        }
    } else if target.contains("wasm") {
        builder = builder.clang_arg("--target=wasm32-unknown-emscripten");
    }

    let bindings = builder.generate().expect("Unable to generate bindings");

    bindings
        .write_to_file(out_dir.join("bindings.rs"))
        .expect("Couldn't write bindings!");
}
