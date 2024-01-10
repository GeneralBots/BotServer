/*****************************************************************************\
|  █████  █████ ██    █ █████ █████   ████  ██      ████   █████ █████  ███ ® |
| ██      █     ███   █ █     ██  ██ ██  ██ ██      ██  █ ██   ██  █   █      |
| ██  ███ ████  █ ██  █ ████  █████  ██████ ██      ████   █   █   █    ██    |
| ██   ██ █     █  ██ █ █     ██  ██ ██  ██ ██      ██  █ ██   ██  █      █   |
|  █████  █████ █   ███ █████ ██  ██ ██  ██ █████   ████   █████   █   ███    |
|                                                                             |
| General Bots Copyright (c) pragmatismo.com.br. All rights reserved.         |
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

import React from "react"
import { SuperSEO } from 'react-super-seo';

class SEO extends React.Component {
  render() {
    let output = "";
    if (this.props.instance) {
      output = (
        <SuperSEO
          title={this.props.instance.title}
          description={this.props.instance.description}
          lang="en"
          openGraph={{
            ogImage: {
              ogImage: this.props.instance.paramLogoImageUrl,
              ogImageAlt: this.props.instance.paramLogoImageAlt,
              ogImageWidth: this.props.instance.paramLogoImageWidth,
              ogImageHeight: this.props.instance.paramLogoImageHeight,
              ogImageType: this.props.instance.paramLogoImageType,
            },
          }}
          twitter={{
            twitterSummaryCard: {
              summaryCardImage: this.props.instance.paramLogoImageUrl,
              summaryCardImageAlt: this.props.instance.paramLogoImageAlt,
              summaryCardSiteUsername: this.props.instance.paramTwitterUsername,
            },
          }}
        />);
    } else {
      output = <div />;
    }
    return output;
  }
}

export default SEO