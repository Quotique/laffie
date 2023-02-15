#!/bin/bash

DEPLOY_HOST="10.110.110.1"
DEPLOY_USER="laffie"

cargo build --release --target aarch64-unknown-linux-gnu;

echo "STOPING SERVICE"
ssh $DEPLOY_USER@$DEPLOY_HOST systemctl --user stop laffie.service

echo "COPYING EXECUTION FILE"
rsync -zav --rsync-path="mkdir -p /home/$DEPLOY_USER/bin && rsync"\
    target/aarch64-unknown-linux-gnu/release/tgbot $DEPLOY_USER@$DEPLOY_HOST:~/bin/;
echo "COPYING CONFIG FILE"
scp -r config $DEPLOY_USER@$DEPLOY_HOST:~;
echo "COPYING SYMBOLS FILE"
rsync -zav --delete symbols $DEPLOY_USER@$DEPLOY_HOST:~;
echo "COPYING SYSTEMD FILE"
rsync -zav --rsync-path="mkdir -p /home/$DEPLOY_USER/.config/systemd/user && rsync"\
    deploy/laffie.service $DEPLOY_USER@$DEPLOY_HOST:~/.config/systemd/user/;

echo "UPDATE TOKEN"
ssh $DEPLOY_USER@$DEPLOY_HOST ./secret.sh;
echo "RELOAD DAEMON"
ssh $DEPLOY_USER@$DEPLOY_HOST systemctl --user daemon-reload;
echo "STARTING SERVICE"
ssh $DEPLOY_USER@$DEPLOY_HOST systemctl --user start laffie.service;
