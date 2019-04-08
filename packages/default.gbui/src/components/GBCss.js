/*****************************************************************************\
|                                               ( )_  _                       |
|    _ _    _ __   _ _    __    ___ ___     _ _ | ,_)(_)  ___   ___     _     |
|   ( '_`\ ( '__)/'_` ) /'_ `\/' _ ` _ `\ /'_` )| |  | |/',__)/' _ `\ /'_`\   |
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

import React from "react";
import { Helmet } from "react-helmet";

class GBCss extends React.Component {
  render() {
    let css = "";
    if (this.props.instance) {
      css = (
        <Helmet>
          <link rel="stylesheet" type="text/css" href={"/themes/" + this.props.instance.theme + "/css/ChatPane.css"} />
          <link rel="stylesheet" type="text/css" href={"/themes/" + this.props.instance.theme + "/css/Content.css"} />
          <link rel="stylesheet" type="text/css" href={"/themes/" + this.props.instance.theme + "/css/Footer.css"} />
          <link rel="stylesheet" type="text/css" href={"/themes/" + this.props.instance.theme + "/css/GifPlayer.css"} />
          <link rel="stylesheet" type="text/css" href={"/themes/" + this.props.instance.theme + "/css/MediaPlayer.css"} />
          <link rel="stylesheet" type="text/css" href={"/themes/" + this.props.instance.theme + "/css/NavBar.css"} />
          <link rel="stylesheet" type="text/css" href={"/themes/" + this.props.instance.theme + "/css/App.css"} />
          <link rel="stylesheet" type="text/css" href={"/themes/" + this.props.instance.theme + "/css/SideBarMenu.css" } />
        </Helmet>
      );
    } else {
      css = <div />;
    }
    return css;
  }
}

export default GBCss;
