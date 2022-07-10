rsync -rav ~/WorkSpace/RaspberryPi pi@192.168.1.5:~/workspace/ --exclude "node_modules" --exclude ".git" --exclude "*.log"
