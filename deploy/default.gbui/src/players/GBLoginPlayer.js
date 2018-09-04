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
import { UserAgentApplication } from "msal";

class GBLoginPlayer extends React.Component {

  constructor(tenant) {
    super();
    this.state = {
      token: "",
    };
  }

  

  login() {

    let config = {
    tenant: "pragmatismo.onmicrosoft.com", //"6ecb2a67-15af-4582-ab85-cc65096ce471",
    signUpSignInPolicy: "b2c_1_susi",
    clientID: '47cbaa05-dbb4-46f8-8608-da386c5131f1'}


    let authority = "https://login.microsoftonline.com/tfp/" +
      config.tenant + "/" +
      config.signUpSignInPolicy;

    let userAgentApplication = new UserAgentApplication(
      config.clientID, authority,
      function (errorDesc, token, error, tokenType) {
        console.log(token);
      }
    );

    let graphScopes = ["Directory.AccessAsUser.All"];

    userAgentApplication.loginPopup(graphScopes).then(function (idToken) {
      userAgentApplication.acquireTokenSilent(graphScopes).then(function (accessToken) {
        console.log(accessToken);

      }, function (error) {
        userAgentApplication.acquireTokenPopup(graphScopes).then(function (accessToken) {
          console.log(accessToken);

        }, function (error) {
          console.log(error);
        });
      })
    }, function (error) {
      console.log(error);
    });
  }

  play() {

  }

  render() {
    return (
      <button
        value="Login"
        onClick={this.login}
      />
    );
  }
}

export default GBLoginPlayer;
