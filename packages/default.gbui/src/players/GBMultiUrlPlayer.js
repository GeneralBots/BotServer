/*****************************************************************************\
|  █████  █████ ██    █ █████ █████   ████  ██      ████   █████ █████  ███ ® |
| ██      █     ███   █ █     ██  ██ ██  ██ ██      ██  █ ██   ██  █   █      |
| ██  ███ ████  █ ██  █ ████  █████  ██████ ██      ████   █   █   █    ██    |
| ██   ██ █     █  ██ █ █     ██  ██ ██  ██ ██      ██  █ ██   ██  █      █   |
|  █████  █████ █   ███ █████ ██  ██ ██  ██ █████   ████   █████   █   ███    |
|                                                                             |
| General Bots Copyright (c) pragmatismo.cloud. All rights reserved.         |
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
| "General Bots" is a registered trademark of pragmatismo.cloud.             |
| The licensing of the program under the AGPLv3 does not imply a              |
| trademark license. Therefore any rights, title and interest in              |
| our trademarks remain entirely with us.                                     |
|                                                                             |
\*****************************************************************************/

import React, { Component } from 'react';

class RenderItem extends Component {
  send(item) {
    setTimeout(() => {
      window.botConnection.postActivity({
        type: 'event',
        name: 'answerEvent',
        data: item.questionId,
        locale: 'en-us',
        textFormat: 'plain',
        timestamp: new Date().toISOString(),
        from: window.user
      });
    }, 400);
  }

  render() {
    return (
      <div className="gb-video-player-wrapper">
        {this.props.list.map(item => (
          <iframe
            title="Video"
            ref="video"
            className="gb-video-react-player"
            src={item.url}
            width="100%"
            height="100%"
          />
        ))}
      </div>
    );
  }
}

class GBMultiUrlPlayer extends Component {
  constructor() {
    super();
    this.state = {
      list: []
    };
  }

  play(data) {
    this.setState({ list: data });
  }

  stop() {
    this.setState({ list: [] });
  }

  render() {
    return (
      <div className="gb-bullet-player" ref={i => (this.playerText = i)}>
        <RenderItem app={this.props.app} list={this.state.list} ref={i => (this.playerList = i)} />
      </div>
    );
  }
}

export default GBMultiUrlPlayer;
