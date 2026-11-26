# REL-3 續：Windows 編不過（小的，擋住發佈）

`ws/rel-3` 的預演（CI run `45765469750`）：Linux 的檢查、macOS 的建置都過了，**Windows 的「compile the product」編不過**：

```
error[E0433]: cannot find `unix` in `os`
 --> tests\harness\src\process.rs:3:14
3 | use std::os::unix::process::ExitStatusExt as _;
error[E0599]: no method named `signal` found for reference `&ExitStatus`
 --> tests\harness\src\process.rs:88:30
error: could not compile `test-harness` (lib) due to 2 previous errors
```

這是 M6a 合進 `dev` 帶進來的（測試程序收乾淨那一段），不是 REL-3 的改動；但 `dev` 現在發佈不出去，所以在這個分支一起修。完整紀錄在你工作區的 `target/ci-logs/rel3-ci-failed.log`。

Windows 的建置一出錯就停（`waiting for other jobs to finish`），所以後面可能還有別的地方一樣只在 Unix 編得過。

## 要做到

1. **把整個工作區裡只有 Unix 才有的寫法全部找出來**，不只上面兩個：`std::os::unix`、`tokio::signal::unix`、`ExitStatusExt`、`CommandExt`、`process_group`、`PermissionsExt`、`MetadataExt`、`libc::`、`nix::` 等。每一個都確認：上面有 `#[cfg(unix)]`（或在只有 Unix 會編的模組裡），而且 Windows 那邊有對應的寫法，行為一樣或說得出為什麼不需要（例如：Windows 沒有訊號，程式被結束時的結束碼怎麼算；Windows 怎麼收掉整個子程序樹）。Windows 建置會編的範圍看 `.github/workflows/ci.yml` 的 `build-windows`（`--lib --bins`，排除清單 `$skip`），`tests/harness` 的 `bin/` 也在裡面。
2. `ci.yml` 的 Windows 建置加上 `--keep-going`，一次就列出所有編不過的地方，不要修一個、推一次。
3. **`build-windows`、`build-macos` 改成每一個 `ws/*` 分支推上去都跑**（現在只有 `dev`、`main`、`ws/rel-*`）。這就是「只有一種系統編得過」擋在合併之前的那道關：功能分支推上去，三種系統的編譯跟 Linux 的檢查同時跑。`release-dev` 照舊只在 `dev`。`docs/testing.md`（或 CI 的說明文件）寫一句：每個工作分支推上去都會在三種系統上編譯。
4. 不准把 Windows 要編的東西加進排除清單來躲錯誤；不准用 `#[cfg(unix)]` 把整個功能在 Windows 上拿掉而不給對應的寫法。

## 驗證和回報

本機是 macOS，編不了 Windows；所以第 1 點要一個一個看過、在回報裡列出每一處和 Windows 那邊怎麼處理。`make check` 全綠就 commit，訊息開頭 `REL-3：`。`git merge dev`、`make check`；簡短回報。我推 `ws/rel-3` 讓 Windows 實際編一次。

你是一次性執行的：回合一結束，背景工作就會被收掉。`make check` 用前景跑、timeout 設長（30 分鐘）。不要每改一點就跑完整的 `make check`：改的時候跑相關的測試和 `make clippy`；完整檢查失敗在某一步，只重跑那一步（例如 `make ironlaws`），全部過了再跑一次完整的；最後先 `git merge dev` 再跑那一次。只在自己的工作區做，暫存檔放工作區的 `target/`，不改 `spec/`。不准改寫歷史、不准 push、不要用 `gh`、不准用 `pkill` 或 `killall`。不准在這台電腦上跑真的 Docker。遇到「Operation not permitted」是沙箱在擋，不要繞過。
