/*****************************************************************************\
|  █████  █████ ██    █ █████ █████   ████  ██      ████   █████ █████  ███ ® |
| ██      █     ███   █ █     ██  ██ ██  ██ ██      ██  █ ██   ██  █   █      |
| ██  ███ ████  █ ██  █ ████  █████  ██████ ██      ████   █   █   █    ██    |
| ██   ██ █     █  ██ █ █     ██  ██ ██  ██ ██      ██  █ ██   ██  █      █   |
|  █████  █████ █   ███ █████ ██  ██ ██  ██ █████   ████   █████   █   ███    |
|                                                                             |
| General Bots Copyright (c) pragmatismo.com.br. All rights reserved.             |
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
| "General Bots" is a registered trademark of pragmatismo.com.br.             |
| The licensing of the program under the AGPLv3 does not imply a              |
| trademark license. Therefore any rights, title and interest in              |
| our trademarks remain entirely with us.                                     |
|                                                                             |
\*****************************************************************************/

import React, { Component } from "react";

class GBMarkdownPlayer extends Component {

  send(value) {
    setTimeout(() => {
      window.botConnection
        .postActivity({
          type: "event",
          name: "quality",
          data: value,
          locale: "en-us",
          textFormat: "plain",
          timestamp: new Date().toISOString(),
          from: { id: "webUser", name: "You" }
        })
    }, 400);
  }

  sendAnswer(text) {
    setTimeout(() => {
      window.botConnection
        .postActivity({
          type: "event",
          name: "answerEvent",
          data: text,
          locale: "en-us",
          textFormat: "plain",
          timestamp: new Date().toISOString(),
          from: window.user
        })
    }, 400);

  }


  constructor() {
    super();
    this.state = {
      content: "",
      prevId: 0,
      nextId: 0
    };
  }

  play(data) {
    this.setState({ content: data.content, prevId: data.prevId, nextId: data.nextId });
  }

  stop() {
    this.setState({ content: "" });
  }

  createMarkup() {
    return { __html: this.state.content };
  }

  clickYes() {
    this.send(1);
  }

  clickNo() {
    this.send(0);
  }

  render() {

    var quality =
      <div className="gb-markdown-player-quality">
        <span ref={i => (this.quality = i)}>Is the answer OK?</span>
        &nbsp;&nbsp;
        <button className="gb-quality-button-yes" onClick={() => this.clickYes()} ref={i => (this.Yes = i)}>
          Yes
        </button>
        &nbsp;|&nbsp;
        <button className="gb-quality-button-no" onClick={() => this.clickNo()} ref={i => (this.No = i)}>
          No
        </button>
      </div>;

    var next = "", prev = "";

    if (this.state.content === "") {
      quality = "";
    }

    if (this.state.prevId) {
      prev = <button style={{ color: 'blue', cursor: 'pointer' }}
        onClick={() => this.sendAnswer(this.state.prevId)}>
        Back
      </button>
    }
    if (this.state.nextId) {
      next = <button style={{ color: 'blue', cursor: 'pointer' }}
        onClick={() => this.sendAnswer(this.state.nextId)}>
        Next
      </button>
    }

    return (
      <div ref={i => (this.playerText = i)} className="media-player">
        <div className="media-player-container">
          <div className="media-player-scroll">
            <div><span>{prev}</span>&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;<span>{next}</span></div>
            <span dangerouslySetInnerHTML={this.createMarkup()} />
          </div>
        </div>
        {quality}
      </div>
    );
  }
}

export default GBMarkdownPlayer;
