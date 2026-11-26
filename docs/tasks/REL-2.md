# REL-2：第一次真的發佈開發版失敗的兩件事（小的）

REL-1 合進 `dev` 之後，第一次推 `dev` 就真的走了開發版那條路（CI run `45749213973`）：三種系統的檢查、六個打包工作全部通過，**失敗在發佈工作的「push one tag for both architectures」**。紀錄在主資料夾的 `target/ci-logs/dev-first-publish-failed.log`（你的工作區沒有，內容見下）：

```
ERROR: annotations are not supported for Docker manifest lists; use "oci-mediatypes=true" when building the source images
```

失敗在換頁、移 `:dev` 之前，所以沒有產生半套的頁面，`:dev` 也沒動。上一輪的審查就預告過：建置時設了 `oci-mediatypes=true`，但映像存成 tar（`docker save`）、在發佈工作讀回來（`docker load`）再推上去，runner 的 Docker 如果用的是傳統的映像存放方式，讀回來就變回 Docker 格式，推上去的單一架構映像就不是 OCI，組索引時加註解就失敗。

你在工作區 `~/Documents/AnyBotNext-wt/rel1`，分支 `ws/rel-2`，剛從 `dev` 切出來。

## 一、組多架構映像（擋著每一次開發版發佈）

要做到：

1. 推上去的每一個單一架構映像、組出來的索引，都不依賴「runner 的 Docker 恰好用哪一種存放方式」。查 Docker 和 docker/build-push-action 的官方文件，選一種穩的做法，寫在回報裡。可以考慮的方向（你判斷）：
   - 不經過 `docker save`／`docker load`：在建置工作裡直接把 OCI 格式的映像推到這次專用的名字（照現在「推了才知道摘要」的規則，摘要從推送的輸出拿），發佈工作只用摘要組索引；
   - 或者不在索引上加註解，commit 改由映像本身的標籤（`org.opencontainers.image.revision`、`org.opencontainers.image.version`，建置時寫進去）記錄。
2. `move-latest.sh`、`move-dev.sh` 也有加註解或讀註解的地方，一起照同一個做法改。`latest` 的「只准往前」要讀得到現在 `latest` 的版本：從哪裡讀，跟上面的做法一致。
3. 預演（推 `ws/rel-*`）照樣不推任何東西。所以這一步在預演裡驗不到真的推送；自我測試的替身要照真的 Docker 的行為演：給它一個不是 OCI 的來源時，加註解就失敗（現在的替身什麼都收，所以這個錯在自我測試裡抓不到）。證明會失敗。

## 二、Intel Mac 做 DMG 偶爾失敗

第九次預演，Intel Mac 那一個打包工作失敗：`hdiutil: couldn't eject "disk2" - Resource busy`。重跑那一個工作就過了；第四到第八次預演也都過。這是 GitHub 的 Mac 機器上偶爾有別的程式佔著磁碟映像，但它會讓推 `dev` 偶爾整個變紅。

- 做 DMG 那一步，失敗訊息是這一種（`Resource busy`）時，等幾秒再重試，最多 3 次；別的錯誤照樣直接失敗。重試的次數和等待寫在腳本裡，每一次重試印一行。
- 查 Tauri 和 `hdiutil` 的文件或已知的問題，看有沒有比重試更根本的做法（例如做 DMG 前先停掉 Spotlight 對那個資料夾的索引）；有就一起做，沒有就只做重試。

## 驗證和回報

- `make check` 全綠就 commit；`git merge dev`、`make check`；簡短回報（兩項怎麼改的、查到的文件、自我測試的替身怎麼演、「會失敗」的表）。
- 我會推 `ws/rel-2` 跑預演，綠了就合進 `dev`，再推 `dev` 真的試一次。

你是一次性執行的：回合一結束，背景工作就會被收掉。`make check` 用前景跑、timeout 設長（30 分鐘）。只在自己的工作區做，暫存檔放工作區的 `target/`，不改 `spec/`。不准改寫歷史、不准 push、不要用 `gh`（測試裡的替身除外）、不准用 `pkill` 或 `killall`。不准在這台電腦上跑真的 Docker。發佈的條件只准變嚴、不准放寬。遇到「Operation not permitted」是沙箱在擋，不要繞過。
