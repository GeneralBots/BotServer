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

import React, { Component } from "react";
import { Report } from "react-powerbi";
import * as pbi from "powerbi-client";

class GBPowerBIPlayer extends Component {
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
        .subscribe(console.log("success"));
    }, 400);
  }

  constructor() {
    super();

    this.state = {
      embedUrl: "",
      accessToken: "",
      filters: null
    };
  }

  play(data) {
    var jobId = 1;

    const basicFilter = {
      $schema: "http://powerbi.com/product/schema#basic",
      target: {
        table: "PauiniJob",
        column: "jobId"
      },
      operator: "Equals",
      values: [jobId],
      filterType: pbi.models.FilterType.BasicFilter
    };

    this.setState({
      embedUrl: data.embedUrl,
      accessToken: data.accessToken,
      filters: [basicFilter]
    });
  }

  stop() {
    this.setState({
      embedUrl: "",
      accessToken: "",
      filters: null
    });
  }

  clickYes() {
    this.send(1);
  }

  clickNo() {
    this.send(0);
  }

  render() {
    var quality = (
      <div className="gb-markdown-player-quality">
        <span ref={i => (this.quality = i)}>O relatório atende?</span>
        &nbsp;&nbsp;
        <button
          className="gb-quality-button-yes"
          onClick={() => this.clickYes()}
          ref={i => (this.Yes = i)}
        >
          Sim
        </button>
        &nbsp;|&nbsp;
        <button
          className="gb-quality-button-no"
          onClick={() => this.clickNo()}
          ref={i => (this.No = i)}
        >
          Não
        </button>
      </div>
    );

    if (this.state.content === "") {
      quality = "";
    }

    return (
      <div ref={i => (this.playerText = i)} className="media-player">
        <div className="media-player-container">
          <div className="media-player-scroll">
            <Report
              embedUrl={this.state.embedUrl}
              accessToken={this.state.accessToken}
              filterPaneEnabled={true}
              filters={this.state.filters}
              navContentPaneEnabled={false}
            />
          </div>
        </div>
        {quality}
      </div>
    );
  }
}

export default GBPowerBIPlayer;
