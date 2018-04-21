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

class SideBarMenu extends React.Component {
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
      .subscribe(console.log("success"));
  }

  render() {
    return (
      <div>
        <div className="tittleSideBarMenu">
        <img
        className="pragmatismoLogo"
        src={"/themes/" + this.props.instance.theme + "/images/logo.png"}
        alt="General Bots Logo"
      />

        </div>
        <div className="SidebarMenu">
          <div className="IconsMenu">
            <div className="iconMenu">
              <span className="iconText" onClick={() => this.send("showFAQ")}>
                Perguntas frequentes
              </span>
            </div>
            <div className="iconMenu">
              <span className="iconText" onClick={() => this.send("whoAmI")}>
                Quem é você?
              </span>
            </div>
            <div className="iconMenu">
              <span
                className="iconText"
                onClick={() => this.send("showSubjects")}
              >
                Assuntos
              </span>
            </div>
            <div className="iconMenu">
              <span
                className="iconText"
                onClick={() => this.send("giveFeedback")}
              >
                Sugestão
              </span>
            </div>
          </div>
        </div>
      </div>
    );
  }
}

export default SideBarMenu;
