package main

import (
	"encoding/json"
	"errors"
	"fmt"
	"io"
	"io/ioutil"
	"log"
	"net/http"
	"os"
	"os/exec"
	"runtime"
	"strconv"
	"strings"
	"syscall"
	"time"
)

type versionResponse struct {
	Version   string `json:"version"`
	Name      string `json:"name"`
	Changelog struct {
		Features []string `json:"features"`
		Fixes    []string `json:"fixes"`
	}
	Download struct {
		Windows string `json:"windows"`
		Linux   string `json:"linux"`
		Macos   string `json:"macos"`
	}
}

type launcherConfig struct {
	Alpha bool `json:"alpha"`
	Debug bool `json:"debug"`
}

func main() {
	//println(runtime.GOARCH)
	if !hasArg("--headless") {
		println("Lilith Launcher Stable Release 3")
		println("================================")
	}

	config := launcherConfig{
		Alpha: false,
		Debug: false,
	}

	//err := os.Setenv("HTTP_PROXY", "http://10.177.177.95:3128")
	//handle(err)
	//err = os.Setenv("HTTPS_PROXY", "http://10.177.177.95:3128")
	//handle(err)

	homedir, err := os.UserHomeDir()
	handle(err)
	ldir := homedir + "/LilithLauncher"
	ldirConfig := ldir + "/config.json"

	if _, err := os.Stat(ldir); errors.Is(err, os.ErrNotExist) {
		err := os.Mkdir(ldir, os.ModePerm)
		handle(err)
	} else {
		_, err := os.Stat(ldirConfig)
		if err == nil {
			//println("Reading")
			data, err := os.ReadFile(ldirConfig)
			handle(err)
			//println(string(data))
			err = json.Unmarshal(data, &config)
			handle(err)
			//println(config.Debug)
		}
	}

	var url string
	if config.Alpha {
		url = "https://api.lilithmod.xyz/versions/alpha"
	} else {
		url = "https://api.lilithmod.xyz/versions/latest"
	}

	resp, err := http.Get(url)
	handle(err)
	body, err := ioutil.ReadAll(resp.Body)
	handle(err)

	var f versionResponse
	err = json.Unmarshal(body, &f)
	handle(err)

	var download string
	switch runtime.GOOS {
	case "windows":
		download = f.Download.Windows
	case "darwin":
		download = f.Download.Macos
	case "linux":
		download = f.Download.Linux
	default:
		download = f.Download.Linux
	}

	filename := download[strings.LastIndex(download, "/")+1:]
	//println(filename)

	dir, err := os.ReadDir(ldir)
	handle(err)

	path := ""
	for _, v := range dir {
		if v.Name() == filename {
			path = ldir + "/" + v.Name()
		}
	}

	if path == "" {
		println("Couldn't find the latest Lilith version, downloading...")
		err := DownloadFile(ldir+"/"+filename, download)
		handle(err)
		println("\r100% Downloaded")
		path = ldir + "/" + filename
	}

	if runtime.GOOS != "windows" {
		cmd := exec.Command("chmod", "+x", path)
		err := cmd.Run()
		handle(err)
	}

	deathCount := 0
	for {
		if deathCount > 4 {
			println("Relaunched too many times, shutting down...")
			break
		}
		var cmd *exec.Cmd
		if config.Debug {
			println("Launching Lilith in debug mode")
			cmd = exec.Command(path, "--dev", "--iknowwhatimdoing")
		} else {
			cmd = exec.Command(path, "--iknowwhatimdoing")
		}

		cmd.Stdout = os.Stdout
		cmd.Stderr = os.Stderr
		err := cmd.Run()
		if err != nil {
			if strings.Contains(err.Error(), "valid Win32 application") || strings.Contains(err.Error(), "segmentation") || deathCount == 4 {
				println("Failed to launch Lilith, deleting...")
				err := os.Remove(path)
				handle(err)
				path, err := os.Executable()
				handle(err)
				err = syscall.Exec(path, []string{os.Args[0], "--headless"}, os.Environ())
				handle(err)
			}
		}
		deathCount++
	}

}

func handle(err error) {
	if err != nil {
		log.Fatalln(err)
	}
}

func hasArg(str string) bool {
	return isElementExist(os.Args, str)
}

func isElementExist(s []string, str string) bool {
	for _, v := range s {
		if v == str {
			return true
		}
	}
	return false
}

func DownloadFile(dest string, url string) error {

	out, err := os.Create(dest)

	//if err != nil {
	//	fmt.Println(path.String())
	//	panic(err)
	//}

	defer out.Close()

	headResp, err := http.Head(url)

	if err != nil {
		panic(err)
	}

	defer headResp.Body.Close()

	size, err := strconv.Atoi(headResp.Header.Get("Content-Length"))

	if err != nil {
		panic(err)
	}

	done := make(chan int64)

	go PrintDownloadPercent(done, dest, int64(size))

	resp, err := http.Get(url)

	if err != nil {
		panic(err)
	}

	defer resp.Body.Close()

	n, err := io.Copy(out, resp.Body)

	done <- n

	return err
}

func PrintDownloadPercent(done chan int64, path string, total int64) {
	var stop bool = false
	file, err := os.Open(path)
	if err != nil {
		log.Fatal(err)
	}
	defer file.Close()
	for {
		select {
		case <-done:
			stop = true
		default:
			fi, err := file.Stat()
			if err != nil {
				log.Fatal(err)
			}

			size := fi.Size()
			if size == 0 {
				size = 1
			}

			var percent float64 = float64(size) / float64(total) * 100
			fmt.Printf("\r%.0f", percent)
			print("% Downloaded")
		}

		if stop {
			break
		}
		time.Sleep(time.Second)
	}
}

//func printValue(k string, v interface{}, p string) {
//	switch vv := v.(type) {
//	case string:
//		println(p + k + ": " + vv)
//	case []interface{}:
//		if len(vv) == 0 {
//			println(p + k + ": []")
//		} else {
//			println(p + k + ": [")
//			for i, u := range vv {
//				printValue(strconv.Itoa(i), u, p+"  ")
//			}
//			println(p + "]")
//		}
//
//	case map[string]interface{}:
//		println(p + k + ": {")
//		printValues(vv, p+"  ")
//		println(p + "}")
//
//	}
//}
//
//func printValues(m map[string]interface{}, p string) {
//	for k, v := range m {
//		printValue(k, v, p)
//	}
//}
