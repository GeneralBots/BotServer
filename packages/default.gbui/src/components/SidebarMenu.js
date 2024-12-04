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

import React from "react";

class SideBarMenu extends React.Component {
  send(command) {
    window.botConnection
      .postActivity({
        type: "event",
        name: command,
        locale: "en-us",
        textFormat: "plain",
        timestamp: new Date().toISOString()
      });
  }

  render() {
    return (
      <div>
        <div className="titleSideBarMenu">
          <img
            className="pragmatismoLogo"
            src={this.props.instance.logo}
            alt="General Bots Logo" />

        </div>
        <div className="SidebarMenu">
          <div className="IconsMenu">
            <div className="iconMenu">
              <span className="iconText" onClick={() => this.send("showFAQ")}>
                FAQ
              </span>
            </div>
            <div className="iconMenu">
              <span className="iconText"
                onClick={() => window.open(`https://pragmatismo.sharepoint.com/sites/bots/Online/${this.props.instance.botId}.gbai`)}
              >
                Drive
              </span>
            </div>
            <div className="iconMenu">
              <span
                className="iconText"
                onClick={() => this.send("showSubjects")}>
                Subjects
              </span>
            </div>
            <div className="iconMenu">
              <span
                className="iconText"
                onClick={() => window.open('mailto:talk@pragmatismo.cloud')}
              >
                Suggestions
              </span>

            </div>
          </div>
        </div>
      </div>
    );
  }
}

export default SideBarMenu;
