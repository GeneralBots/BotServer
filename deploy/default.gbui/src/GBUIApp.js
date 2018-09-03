/*****************************************************************************\
|                                               ( )_  _                       |
|    _ _    _ __   _ _    __    ___ ___     _ _ | ,_)(_)  ___   ___     _     |
|   ( '_`\ ( '__)/'_` ) /'_ `\/' _ ` _ `\ /'_` )| |  | |/',__)/' _ `\ /'_`\   |
|   | (_) )| |  ( (_| |( (_) || ( ) ( ) |( (_| || |_ | |\__, \| ( ) |( (_) )  |
|   | ,__/'(_)  `\__,_)`\__  |(_) (_) (_)`\__,_)`\__)(_)(____/(_) (_)`\___/'  |
|   | |                ( )_) |                                                |
|   (_)                 \___/'                                                |
|                                                                             |
| General Bots Copyright (c) Pragmatismo.io. All rights reserved.             |
| Licensed under the AGPL-3.0.                                                |
|                                                                             | 
| According to our dual licensing model, this program can be used either      |
| under the terms of the GNU Affero General Public License, version 3,        |
| or under a proprietary license.                                             |
|                                                                             |
| The texts of the GNU Affero General Public License with an additional       |
| permission and of our proprietary license can be found at and               |
| in the LICENSE file you have received along with this program.              |
|                                                                             |
| This program is distributed in the hope that it will be useful,             |
| but WITHOUT ANY WARRANTY; without even the implied warranty of              |
| MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the                |
| GNU Affero General Public License for more details.                         |
|                                                                             |
| "General Bots" is a registered trademark of Pragmatismo.io.                 |
| The licensing of the program under the AGPLv3 does not imply a              |
| trademark license. Therefore any rights, title and interest in              |
| our trademarks remain entirely with us.                                     |
|                                                                             |
\*****************************************************************************/

import React from "react";
import GBMarkdownPlayer from "./players/GBMarkdownPlayer.js";
import GBImagePlayer from "./players/GBImagePlayer.js";
import GBVideoPlayer from "./players/GBVideoPlayer.js";
import GBLoginPlayer from "./players/GBLoginPlayer.js";
import GBBulletPlayer from "./players/GBBulletPlayer.js";
import SidebarMenu from "./components/SidebarMenu.js";
import GBCss from "./components/GBCss.js";
import { DirectLine } from "botframework-directlinejs";
import { ConnectionStatus } from "botframework-directlinejs";
import { SpeechRecognizer } from "botframework-webchat/CognitiveServices";
import { SpeechSynthesizer } from "botframework-webchat/CognitiveServices";
import { SynthesisGender } from "botframework-webchat/CognitiveServices";
import { Chat } from "botframework-webchat";
import GBPowerBIPlayer from "./players/GBPowerBIPlayer.js";

class GBUIApp extends React.Component {
  constructor() {
    super();

    this.state = {
      botConnection: null,
      instance: null,
      token: null,
      instanceClient: null
    };
  }

  send(command) {
    window.botConnection
      .postActivity({
        type: "event",
        name: command,
        locale: "en-us",
        textFormat: "plain",
        timestamp: new Date().toISOString(),
        from: { id: "webUser", name: "You" }
      })
      .subscribe(console.log("EVENT SENT TO Guaribas."));
  }
  getUser() {
    return { id: "webUser@gb", name: "You" };
  }

  postEvent(name, value) {
    window.botConnection.postActivity({
      type: "event",
      value: value,
      from: this.getUser(),
      name: name
    });
  }

  postMessage(value) {
    window.botConnection.postActivity({
      type: "message",
      text: value,
      from: this.getUser()
    });
  }

  configureChat() {
    var botId = window.location.href.split("/")[3];

    if (!botId) {
      botId = "[default]";
    }

    fetch("/instances/" + botId)
      .then(res => res.json())
      .then(
        result => {
          this.setState({instanceClient:result});
          this.setupBotConnection();
        },
        error => {
          this.setState({
            isLoaded: false,
            err: error
          });
        }
      );
  }

  setupBotConnection() {
    let _this_ = this;
    window["botchatDebug"] = true;

    const botConnection = new DirectLine({
      secret: this.state.instanceClient.secret
    });

    botConnection.connectionStatus$.subscribe(connectionStatus => {
      if (connectionStatus === ConnectionStatus.Online) {
        botConnection.postActivity({
          type: "event",
          value: "startGB",
          from: this.getUser(),
          name: "startGB"
        });

        _this_.setState({ botConnection: botConnection });
      }
    });

    window.botConnection = botConnection;
    this.postEvent("startGB", true);

    botConnection.activity$
      .filter(
        activity =>
          activity.type === "event" && activity.name === "loadInstance"
      )
      .subscribe(activity => {
        _this_.setState({ instance: activity.value });
      });
      
    botConnection.activity$
      .filter(activity => activity.type === "event" && activity.name === "stop")
      .subscribe(activity => {
        if (_this_.player) {
          _this_.player.stop();
        }
      });

    botConnection.activity$
      .filter(activity => activity.type === "event" && activity.name === "play")
      .subscribe(activity => {
        _this_.setState({ playerType: activity.value.playerType });
        _this_.player.play(activity.value.data);
      });
  }

  componentDidMount() {
    this.configureChat();
  }

  render() {
    

    let playerComponent = "";

    if (this.state.playerType) {
      switch (this.state.playerType) {
        case "markdown":
          playerComponent = (
            <GBMarkdownPlayer
              app={this}
              ref={player => {
                this.player = player;
              }}
            />
          );
          break;
        case "bullet":
          playerComponent = (
            <GBBulletPlayer
              app={this}
              ref={player => {
                this.player = player;
              }}
            />
          );
          break;
        case "video":
          playerComponent = (
            <GBVideoPlayer
              app={this}
              ref={player => {
                this.player = player;
              }}
            />
          );
          break;
        case "image":
          playerComponent = (
            <GBImagePlayer
              app={this}
              ref={player => {
                this.player = player;
              }}
            />
          );
          break;
        case "pbi":
          playerComponent = (
            <GBPowerBIPlayer
              app={this}
              ref={player => {
                this.player = player;
              }}
            />
          );
          break;
         case "login":
          playerComponent = (
            <GBLoginPlayer
              app={this}
              ref={player => {
                this.player = player;
              }}
            />
          );
          break;
        default:
          console.log(
            "GBERROR: Unknow player type specified on message from server."
          );
          break;
      }


    }

    let speechOptions;
    let chat = <div />;
    let gbCss =<div />;


    let sideBar = (
      <div className="sidebar">
        <SidebarMenu chat={this.chat} instance={this.state.instance} />
      </div>
    );
    
    if (this.state.botConnection && this.state.instance) {
      let token = this.state.instanceClient.speechToken;
      gbCss = <GBCss instance={this.state.instance} />;

      function getToken() {
        return new Promise((resolve, reject) => {
          resolve(token);
        });
      }
  
      speechOptions = {
        speechRecognizer: new SpeechRecognizer({
          locale: "pt-br",
          fetchCallback: (authFetchEventId) => getToken(),
          fetchOnExpiryCallback: (authFetchEventId) => getToken()
        }),
        speechSynthesizer: new SpeechSynthesizer({
          fetchCallback: (authFetchEventId) => getToken(),
          fetchOnExpiryCallback: (authFetchEventId) => getToken(),
          gender: SynthesisGender.Male,
          voiceName: 'Microsoft Server Speech Text to Speech Voice (pt-BR, Daniel, Apollo)'
        })
      };

      chat = (
        <Chat
          ref={chat => {
            this.chat = chat;
          }}
          locale={'pt-br'}
          botConnection={this.state.botConnection}
          user={this.getUser()}
          bot={{ id: "bot@gb", name: "Bot" }}
          speechOptions={speechOptions}
        />
      );


    }

    if (!this.state.instance) {
      sideBar = "";
    }

    return (
      <div>
        {gbCss}    
        {sideBar}
        <div className="player">{playerComponent}</div>
        {chat}
      </div>
    );
  }
}

export default GBUIApp;
