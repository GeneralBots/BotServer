/*****************************************************************************\
|                                               ( )_  _                       |
|    _ _    _ __   _ _    __    ___ ___     _ _ | ,_)(_)  ___   ___     _     |
|   ( '_`\ ( '__)/'_` ) /'_ `\/' _ ` _ `\ /'_` )| |  | |/',__)/' v `\ /'_`\   |
|   | (_) )| |  ( (_| |( (_) || ( ) ( ) |( (_| || |_ | |\__, \| (˅) |( (_) )  |
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

import React from 'react';
import GBMarkdownPlayer from './players/GBMarkdownPlayer.js';
import GBImagePlayer from './players/GBImagePlayer.js';
import GBVideoPlayer from './players/GBVideoPlayer.js';
import GBLoginPlayer from './players/GBLoginPlayer.js';
import GBBulletPlayer from './players/GBBulletPlayer.js';
import SidebarMenu from './components/SidebarMenu.js';
import GBCss from './components/GBCss.js';
import { DirectLine } from 'botframework-directlinejs';
import { ConnectionStatus } from 'botframework-directlinejs';
import ReactWebChat from 'botframework-webchat';
import { UserAgentApplication } from 'msal';


class GBUIApp extends React.Component {
  constructor() {
    super();

    this.state = {
      line: null,
      instance: null,
      token: null,
      instanceClient: null
    };
    window.user = this.getUser();
  }

  sendToken(token) {
    setTimeout(() => {
      window.line
        .postActivity({
          type: 'event',
          name: 'updateToken',
          data: token,
          locale: 'en-us',
          textFormat: 'plain',
          timestamp: new Date().toISOString(),
          from: this.getUser()
        })
        .subscribe(() => {
          window.userAgentApplication.logout();
          console.log('updateToken done');
        });
    }, 400);
  }

  send(command) {
    window.line
      .postActivity({
        type: 'event',
        name: command,
        locale: 'en-us',
        textFormat: 'plain',
        timestamp: new Date().toISOString(),
        from: this.getUser()
      })
      .subscribe(console.log('EVENT SENT TO Guaribas.'));
  }

  getUser() {

    return { id: 'web@gb', name: 'You' };
  }

  postEvent(name, value) {
    window.line.postActivity({
      type: 'event',
      value: value,
      from: this.getUser(),
      name: name
    });
  }

  postMessage(value) {
    window.line.postActivity({
      type: 'message',
      text: value,
      from: this.getUser()
    });
  }

  configureChat() {
    var botId = window.location.href.split('/')[3];
    if (botId.indexOf('#') !== -1) {
      botId = botId.split('#')[0];
    }

    if (!botId || botId === '') {
      botId = '[default]';
    }

    fetch('/instances/' + botId)
      .then(res => res.json())
      .then(
        result => {
          this.setState({ instanceClient: result });
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

  authenticate() {
    
    if (this.state.instanceClient.authenticatorClientId === null) {
      return;
    }

    let _this_ = this;
    let authority = 'https://login.microsoftonline.com/' + this.state.instanceClient.authenticatorTenant;

    let graphScopes = ['Directory.AccessAsUser.All'];

    let userAgentApplication = new UserAgentApplication(
      this.state.instanceClient.authenticatorClientId,
      authority,
      function (errorDesc, token, error, tokenType) {
        if (error) {
          console.log(error);
        }
      }
    );
    window.userAgentApplication = userAgentApplication;

    if (!userAgentApplication.isCallback(window.location.hash) && window.parent === window && !window.opener) {
      var user = userAgentApplication.getUser();
      if (user) {
        userAgentApplication.acquireTokenSilent(graphScopes).then(
          function (accessToken) {
            _this_.sendToken(accessToken);
          },
          function (error) {
            console.log(error);
          }
        );
      }
    }
  }

  setupBotConnection() {
    let _this_ = this;
    window['botchatDebug'] = true;

    const line = new DirectLine({
      token: this.state.instanceClient.webchatToken
    });

    line.connectionStatus$.subscribe(connectionStatus => {
      if (connectionStatus === ConnectionStatus.Online) {
        _this_.setState({ line: line });
        window['botConnection'] = line;
        line.postActivity({
          type: 'event',
          value: 'startGB',
          from: this.getUser(),
          name: 'startGB'
        });
      }
    });

    window.line = line;
    this.postEvent('startGB', true);

    line.activity$
      .filter(activity => activity.type === 'event' && activity.name === 'loadInstance')
      .subscribe(activity => {
        _this_.setState({ instance: activity.value });
        _this_.authenticate();
      });

    line.activity$
      .filter(activity => activity.type === 'event' && activity.name === 'stop')
      .subscribe(activity => {
        if (_this_.player) {
          _this_.player.stop();
        }
      });

    line.activity$
      .filter(activity => activity.type === 'event' && activity.name === 'play')
      .subscribe(activity => {
        _this_.setState({ playerType: activity.value.playerType });
        _this_.player.play(activity.value.data);
      });
  }

  componentDidMount() {
    this.configureChat();
  }

  render() {
    let playerComponent = '';

    if (this.state.playerType) {
      switch (this.state.playerType) {
        case 'markdown':
          playerComponent = (
            <GBMarkdownPlayer
              app={this}
              ref={player => {
                this.player = player;
              }}
            />
          );
          break;
        case 'bullet':
          playerComponent = (
            <GBBulletPlayer
              app={this}
              ref={player => {
                this.player = player;
              }}
            />
          );
          break;
        case 'video':
          playerComponent = (
            <GBVideoPlayer
              app={this}
              ref={player => {
                this.player = player;
              }}
            />
          );
          break;
        case 'image':
          playerComponent = (
            <GBImagePlayer
              app={this}
              ref={player => {
                this.player = player;
              }}
            />
          );
          break;
        /* case 'pbi':
             playerComponent = (
               <GBPowerBIPlayer
                 app={this}
                 ref={player => {
                   this.player = player;
                 }}
               />
             );
             break; */
        case 'login':
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
          console.log('GBERROR: Unknow player type specified on message from server.');
          break;
      }
    }

    let chat = <div />;
    let gbCss = <div />;

    let sideBar = (
      <div className="sidebar">
        <SidebarMenu chat={this.chat} instance={this.state.instance} />
      </div>
    );

    if (this.state.line && this.state.instance) {
      gbCss = <GBCss instance={this.state.instance} />;

      // let speechOptions;
      // let token = this.state.instanceClient.speechToken;

      // speechOptions = {
      //     speechRecognizer: new SpeechRecognizer({
      //         locale: "pt-br",
      //         fetchCallback: (authFetchEventId) => getToken(),
      //         fetchOnExpiryCallback: (authFetchEventId) => getToken()
      //     }),
      //     speechSynthesizer: new SpeechSynthesizer({
      //         fetchCallback: (authFetchEventId) => getToken(),
      //         fetchOnExpiryCallback: (authFetchEventId) => getToken(),
      //         gender: SynthesisGender.Male,
      //         voiceName: 'Microsoft Server Speech Text to Speech Voice (pt-BR, Daniel, Apollo)'
      //     })
      // };

      chat = (
        <ReactWebChat
          ref={chat => {
            this.chat = chat;
          }}
          locale={'pt-br'}
          directLine={this.state.line}
          user={this.getUser()}
          bot={{ id: 'bot@gb', name: 'Bot' }}
        />
      );
    }

    if (!this.state.instance) {
      sideBar = '';
    }

    return (
      <div>
        {gbCss}
        {sideBar}
        <div className="player">{playerComponent}</div>
        <div className="webchat">
          {chat}
        </div>
      </div>
    );
  }
}

export default GBUIApp;
