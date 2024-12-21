```sh ./a.sh
#!/bin/bash

# Help message function
show_help() {
    echo "Usage: $0 [directory] [output_file]"
    echo
    echo "Aggregates code files into a single markdown file with code blocks"
    echo
    echo "Arguments:"
    echo "  directory    Directory to scan for code files (default: current directory)"
    echo "  output_file  Output file name (default: aggregated_code.md)"
    echo
    echo "Example:"
    echo "  $0 ./my_project output.md"
}

# Default values
dir="${1:-.}"
output_file="${2:-aggregated_code.md}"

# Show help if requested
if [[ "$1" == "-h" ]] || [[ "$1" == "--help" ]]; then
    show_help
    exit 0
fi

# Check if directory exists
if [ ! -d "$dir" ]; then
    echo "Error: Directory '$dir' not found!"
    exit 1
fi

# Clear or create output file
> "$output_file"

# Find and process files
find "$dir" -type f \( -name "*.py" -o -name "*.js" -o -name "*.ts" -o -name "*.jsx" -o -name "*.tsx" -o -name "*.html" -o -name "*.css" -o -name "*.java" -o -name "*.cpp" -o -name "*.c" -o -name "*.sh" \) | while read -r file; do
    # Get the file extension
    ext="${file##*.}"
    
    # Add the markdown code block header
    echo -e "\`\`\`$ext $file" >> "$output_file"
    
    # Add the file content
    cat "$file" >> "$output_file"
    
    # Add the closing code block
    echo -e "\`\`\`\n" >> "$output_file"
done

echo "Files have been aggregated into $output_file"```

```html ./public/index.html
<!--
/*****************************************************************************\
|  █████  █████ ██    █ █████ █████   ████  ██      ████   █████ █████  ███ ® |
| ██      █     ███   █ █     ██  ██ ██  ██ ██      ██  █ ██   ██  █   █      |
| ██  ███ ████  █ ██  █ ████  █████  ██████ ██      ████   █   █   █    ██    |
| ██   ██ █     █  ██ █ █     ██  ██ ██  ██ ██      ██  █ ██   ██  █      █   |
|  █████  █████ █   ███ █████ ██  ██ ██  ██ █████   ████   █████   █   ███    |
|                                                                             |
| General Bots Copyright (c) pragmatismo.cloud. All rights reserved.          |
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
| but WITHOUT ANY WARRANTY, without even the implied warranty of              |
| MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the                |
| GNU Affero General Public License for more details.                         |
|                                                                             |
| "General Bots" is a registered trademark of pragmatismo.cloud.              |
| The licensing of the program under the AGPLv3 does not imply a              |
| trademark license. Therefore any rights, title and interest in              |
| our trademarks remain entirely with us.                                     |
|                                                                             |
\*****************************************************************************/
-->

<!DOCTYPE html>
<html style="width: 100%; height: 100%" class="{themeColor}">
  <head>
    <meta charset="utf-8" />
    <meta name="viewport" content="width=device-width, initial-scale=1" />
    <link rel="shortcut icon" href="%PUBLIC_URL%/favicon.ico" />
    <link rel="stylesheet" type="text/css" href="/themes/{theme}/css/colors.css" />
    <link rel="stylesheet" type="text/css" href="/themes/{theme}/css/default.css" />
    <script src="./js/webchat.js"></script>
    <title>{title} | General Bots</title>
    <script>
      document.addEventListener('DOMContentLoaded', () => {
        setTimeout(() => {
          const input = document.querySelector('.webchat__send-box-text-box__input');
          if (input) {
            input.focus();
          }
        }, 3000); // Adjust timing as needed
      });
    </script>
  </head>

  <body style="background-color: black">
    <div id="root"></div>
  </body>
</html>
```

```js ./src/players/GBUrlPlayer.js
﻿/*****************************************************************************\
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

import React, { Component } from "react";

class GBUrlPlayer extends Component {
  constructor() {
    super();
    this.state = {
      src: ""
    };
  }

  play(url) {
    this.setState({ src: url });
  }

  stop() {
    this.setState({ src: "" });
  }
  render() {
    return (
      <div className="gb-video-player-wrapper">
        <iframe title="Video" ref="video"
          className="gb-video-react-player"
          src={this.state.src}
          width="100%"
          height="100%"
        />
      </div>
    );
  }
}

export default GBUrlPlayer;```

```js ./src/players/GBImagePlayer.js
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

class GBImagePlayer extends Component {
  constructor() {
    super();
    this.state = {};
  }

  play(url) {
    this.playerImage.src = url;
  }

  stop() {
    this.playerImage.src = '';
  }

  render() {
    return (
      <div className="gb-image-player-outter-div" ref={i => (this.playerText = i)}>
        <img
          ref={i => (this.playerImage = i)}
          className="gb-image-player-img"
          src=""
          alt=""
        />
      </div>
    );
  }
}

export default GBImagePlayer;
```

```js ./src/players/GBBulletPlayer.js
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

import React, { Component } from "react";


class RenderItem extends Component {
  send(item) {
    setTimeout(()=>{
    window.botConnection
      .postActivity({
        type: "event",
        name: "answerEvent",
        data: item.questionId,
        locale: "en-us",
        textFormat: "plain",
        timestamp: new Date().toISOString(),
        from: window.user
      })
    },400);
  }

  render() {
    return (
      <ul>
        {this.props.list.map((item) => (
          <li key={item.questionId}>
            <label className="gb-bullet-player-item" onClick={this.send.bind(this, item)}>{item.content}</label>
          </li>
        ))}
      </ul>
    );
  }
}

class GBBulletPlayer extends Component {
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
      <div
        className="gb-bullet-player"
        ref={i => (this.playerText = i)}
      >
        <RenderItem
          app={this.props.app}
          list={this.state.list}
          ref={i => (this.playerList = i)}
        />
      </div>
    );
  }
}

export default GBBulletPlayer;
```

```js ./src/players/GBLoginPlayer.js
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
import { UserAgentApplication } from "msal";

class GBLoginPlayer extends React.Component {
  constructor() {
    super();
    this.state = {
      login: {}
    };
  }

  doLogin() {
    let authority =
      "https://login.microsoftonline.com/" +
      this.state.login.authenticatorTenant;

    let graphScopes = ["Directory.AccessAsUser.All"];

    let userAgentApplication = new UserAgentApplication(
      this.state.login.authenticatorClientId,
      authority,
      function (errorDesc, token, error) {
        if (error) {
          this.setState({ login: error});
        }
      })

    userAgentApplication.loginRedirect(graphScopes);
  }

  play(data) {
    this.setState({ login: data });
  }

  stop() {
    this.setState({ login: [] });
  }

  render() {
    return <button onClick={() => this.doLogin(this.state.login)}>Login</button>;
  }
}

export default GBLoginPlayer;
```

```js ./src/players/GBVideoPlayer.js
﻿/*****************************************************************************\
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

import React, { Component } from "react";

class GBVideoPlayer extends Component {
  constructor() {
    super();
    this.state = {
      src: ""
    };
  }

  play(url) {
    this.setState({ src: url });
    this.refs.video.play();
  }

  stop() {
    this.setState({ src: "" });
  }

  render() {
    return (
      <div className="gb-video-player-wrapper">
        <video ref="video"
          className="gb-video-react-player"
          src={this.state.src}
          width="100%"
          height="100%"
        />
      </div>
    );
  }
}

export default GBVideoPlayer;
```

```js ./src/players/GBMultiUrlPlayer.js
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
            key={item.url} 
            title="Video"
            ref="video"
            className="gb-video-react-player"
            src={`${item.url}?t=${Date.now()}`} 
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
```

```js ./src/players/GBMarkdownPlayer.js
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
```

```js ./src/components/SidebarMenu.js
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
```

```js ./src/components/Footer.js
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

import React from "react"


const footer = () => (
    <div className="footer-container">
      General Bots Community Edition
      <br/>
      <a href="http://pragmatismo.cloud">pragmatismo.cloud</a>
  </div>
);
export default footer```

```js ./src/components/GBCss.js
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
import { Helmet } from "react-helmet";

class GBCss extends React.Component {
  render() {
    let css = "";
    if (this.props.instance) {
      css = (
        <Helmet>
          <link rel="stylesheet" type="text/css" href={"/themes/" + this.props.instance.theme + "/css/colors.css"} />
          <link rel="stylesheet" type="text/css" href={"/themes/" + this.props.instance.theme + "/css/default.css" } />
        </Helmet>
      );
    } else {
      css = <div />;
    }
    return css;
  }
}

export default GBCss;
```

```js ./src/components/Debugger.js
// import * as React from "react";
// import Header from "./Header";
// import HeroList, { HeroListItem } from "./HeroList";
// import Progress from "./Progress";
// import "../../../assets/icon-16.png";
// import "../../../assets/icon-32.png";
// import "../../../assets/icon-80.png";
// import $ from "jquery";

// export interface AppProps {
//   title: string;
//   isOfficeInitialized: boolean;
// }

// export interface AppState {
//   listItems: HeroListItem[];
//   mode: number;
//   conversationText: string;
//   scope: string;
//   state: number;
//   stateInfo: string;
//   inputText: string;
//   messages: string;
// }

// export default class App extends React.Component<AppProps, AppState> {
//   constructor(props, context) {
//     super(props, context);
//     this.state = {
//       mode: 0,
//       listItems: [],
//       conversationText: "",
//       scope: "",
//       state: 0,
//       stateInfo: "",
//       messages: "",
//       inputText: "",
//     };
//   }

//   botId = "dev-rodriguez22";
//   botKey = "starter";
//   host = "https://tender-yak-44.telebit.io";
//   breakpointsMap = {};

//   componentDidMount() {
//     this.setState({
//       listItems: [
//         {
//           icon: "Ribbon",
//           primaryText: "Office integration to Bots",
//         },
//         {
//           icon: "Unlock",
//           primaryText: "Unlock features of General Bots",
//         },
//         {
//           icon: "Design",
//           primaryText: "Create your Bots using BASIC",
//         },
//       ],
//     });
//   }

//   context = async () => {
//     const url = `${this.host}/api/v3/${this.botId}/dbg/getContext`;

//     $.ajax({
//       data: { botId: this.botId, botKey: this.botKey },
//       url: url,
//       dataType: "json",
//       method: "POST",
//     })
//       .done(function (item) {
//         console.log("GBWord Add-in: context OK.");
//         const line = item.line;

//         Word.run(async (context) => {
//           var paragraphs = context.document.body.paragraphs;
//           paragraphs.load("$none");
//           await context.sync();
//           for (let i = 0; i < paragraphs.items.length; i++) {
//             const paragraph = paragraphs.items[i];

//             context.load(paragraph, ["text", "font"]);
//             paragraph.font.highlightColor = null;

//             if (i === line) {
//               paragraph.font.highlightColor = "Yellow";
//             }
//           }
//           await context.sync();
//         });
//       })
//       .fail(function (jqXHR, textStatus, errorThrown) {
//         let x = jqXHR,
//           y = errorThrown;

//         console.log(textStatus);
//       });
//   };

//   setExecutionLine = async (line) => {
//     Word.run(async (context) => {
//       var paragraphs = context.document.body.paragraphs;
//       paragraphs.load("$none");
//       await context.sync();
//       for (let i = 0; i < paragraphs.items.length; i++) {
//         const paragraph = paragraphs.items[i];

//         context.load(paragraph, ["text", "font"]);
//         paragraph.font.highlightColor = null;

//         if (i === line) {
//           paragraph.font.highlightColor = "Yellow";
//         }
//       }
//       await context.sync();
//     });
//   };

//   breakpoint = async () => {
//     let line = 0;

//     Word.run(async (context) => {
//       let selection = context.document.getSelection();
//       selection.load();

//       await context.sync();

//       console.log("Empty selection, cursor.");

//       const paragraph = selection.paragraphs.getFirst();
//       paragraph.select();
//       context.load(paragraph, ["text", "font"]);

//       var paragraphs = context.document.body.paragraphs;
//       paragraphs.load("$none");
//       await context.sync();

//       for (let i = 0; i < paragraphs.items.length; i++) {
//         const paragraph1 = paragraphs.items[i];

//         if (paragraph1 === paragraph) {
//           line = i + 1;
//           paragraph.font.highlightColor = "Orange";
//         }
//       }

//       return context.sync();
//     });

//     const url = `${this.host}/api/v3/${this.botId}/dbg/setBreakpoint`;

//     $.ajax({
//       data: { botId: this.botId, botKey: this.botKey, line },
//       url: url,
//       dataType: "json",
//       method: "POST",
//     })
//       .done(function () {
//         console.log("GBWord Add-in: breakpoint OK.");
//       })
//       .fail(function (jqXHR, textStatus, errorThrown) {
//         let x = jqXHR,
//           y = errorThrown;

//         console.log(textStatus);
//       });
//   };

//   refactor = async () => {
//     let line = 0;

//     let change = 'ssssssssssssssssssss';

//     Word.run(async (context) => {
//       let selection = context.document.getSelection();
//       selection.load();

//       await context.sync();

//       var paragraphs = selection.paragraphs;
//       paragraphs.load("$none");
//       await context.sync();
//       let code = '';
//       for (let i = 0; i < paragraphs.items.length; i++) {

//         const paragraph = paragraphs.items[i];
//         context.load(paragraph, ["text", "font"]);
//         code += paragraph.text;
//       }

//       const url = `${this.host}/api/v3/${this.botId}/dbg/refactor`;

//       $.ajax({
//         data: { botId: this.botId, code: code, change: change },
//         url: url,
//         dataType: "json",
//         method: "POST",
//       })
//         .done(async function (data) {

//           Word.run(async (context) => {
//             var selectedRange = context.document.getSelection();
//             context.load(selectedRange, "text");
//             selectedRange.text = data;

//             await context.sync();
//           });
//         })
//         .fail(function (jqXHR, textStatus, errorThrown) {
//           console.log(textStatus);
//         });

//       return context.sync();
//     });

//   };

//   resume = async () => {
//     const url = `${this.host}/api/v3/${this.botId}/dbg/resume`;

//     $.ajax({
//       data: { botId: this.botId, botKey: this.botKey },
//       url: url,
//       dataType: "json",
//       method: "POST",
//     })
//       .done(function () {
//         console.log("GBWord Add-in: resume OK.");
//         this.setState({ mode: 1 });
//       })
//       .fail(function (jqXHR, textStatus, errorThrown) {
//         let x = jqXHR,
//           y = errorThrown;
//         console.log(textStatus);
//       });
//   };

//   step = async () => {
//     const url = `${this.host}/api/v3/${this.botId}/dbg/step`;

//     $.ajax({
//       data: { botId: this.botId, botKey: this.botKey },
//       url: url,
//       dataType: "json",
//       method: "POST",
//     })
//       .done(function () {
//         console.log("GBWord Add-in: step OK.");
//         this.setState({ mode: 2 });
//       })
//       .fail(function (jqXHR, textStatus, errorThrown) {
//         let x = jqXHR,
//           y = errorThrown;
//         console.log(textStatus);
//       });
//   };

//   stop = async () => {
//     const url = `${this.host}/api/v3/${this.botId}/dbg/stop`;

//     $.ajax({
//       data: { botId: this.botId, botKey: this.botKey },
//       url: url,
//       dataType: "json",
//       method: "POST",
//     })
//       .done(function () {
//         console.log("GBWord Add-in: stop OK.");
//         this.setState({ mode: 0 });
//       })
//       .fail(function (jqXHR, textStatus, errorThrown) {
//         let x = jqXHR,
//           y = errorThrown;
//         console.log(textStatus);
//       });
//   };

//   sendMessage = async (args) => {
//     if (args.keyCode === 13) {
//       const text = args.target.value;
//       const url = `${this.host}/api/v3/${this.botId}/dbg/sendMessage`;

//       $.ajax({
//         data: { botId: this.botId, botKey: this.botKey, text: text },
//         url: url,
//         dataType: "json",
//         method: "POST",
//       })
//         .done(function () {
//           console.log("GBWord Add-in: sendMessage OK.");
//           args.target.value = "";
//         })
//         .fail(function (jqXHR, textStatus, errorThrown) {
//           let x = jqXHR,
//             y = errorThrown;
//           console.log(textStatus);
//         });
//     }
//   };

//   waitFor = (delay) => new Promise((resolve) => setTimeout(resolve, delay));

//   refresh = async () => {
//     const context = await this.context();

//     this.setState({
//       conversationText: context['conversationText'],
//       state: context['state'],
//       messages: context['messages'],
//       scope: context['scope'],
//       mode: context['state']
//     });
//     await this.waitFor(3000);
//     await this.refresh();
//   };


//   debug = async () => {
//     if (this.state.mode === 0) {
//       const url = `${this.host}/api/v3/${this.botId}/dbg/start`;

//       $.ajax({
//         data: { botId: this.botId, botKey: this.botKey, scriptName: "auto" },
//         url: url,
//         dataType: "json",
//         method: "POST",
//       })
//         .done(function () {
//           console.log("GBWord Add-in: debug OK.");
//           this.state.mode = 1;
//         })
//         .fail(function (jqXHR, textStatus, errorThrown) {
//           let x = jqXHR,
//             y = errorThrown;
//           console.log(textStatus);
//         });
//     } else if (this.state.mode === 2) {
//       this.resume();
//     }

//     await this.refresh();
//   };

//   formatCode = async () => {
//     return Word.run(async (context) => {
//       var paragraphs = context.document.body.paragraphs;
//       paragraphs.load("$none");
//       await context.sync();
//       for (let i = 0; i < paragraphs.items.length; i++) {
//         const paragraph = paragraphs.items[i];
//         context.load(paragraph, ["text", "font"]);
//         paragraph.font.highlightColor = null;

//         const words = paragraph.split([" "], true /* trimDelimiters*/, true /* trimSpaces */);
//         words.load(["text", "font"]);
//         await context.sync();
//         var boldWords = [];
//         for (var j = 0; j < words.items.length; ++j) {
//           var word = words.items[j];
//           if (word.text === "TALK" && j == 0) boldWords.push(word);
//           if (word.text === "HEAR" && j == 0) boldWords.push(word);
//           if (word.text === "SAVE" && j == 0) boldWords.push(word);
//           if (word.text === "FIND" && j == 3) boldWords.push(word);
//           if (word.text === "OPEN" && j == 0) boldWords.push(word);
//           if (word.text === "WAIT" && j == 0) boldWords.push(word);
//           if (word.text === "SET" && j == 0) boldWords.push(word);
//           if (word.text === "CLICK" && j == 0) boldWords.push(word);
//           if (word.text === "MERGE" && j == 0) boldWords.push(word);
//           if (word.text === "IF" && j == 0) boldWords.push(word);
//           if (word.text === "THEN" && j == 0) boldWords.push(word);
//           if (word.text === "ELSE" && j == 0) boldWords.push(word);
//           if (word.text === "END" && j == 0) boldWords.push(word);
//           if (word.text === "TWEET" && j == 0) boldWords.push(word);
//           if (word.text === "HOVER" && j == 0) boldWords.push(word);
//           if (word.text === "PRESS" && j == 0) boldWords.push(word);
//           if (word.text === "DO" && j == 0) boldWords.push(word);
//         }
//         for (var j = 0; j < boldWords.length; ++j) {
//           boldWords[j].font.color = "blue";
//           boldWords[j].font.bold = true;
//         }
//       }
//       await context.sync();
//     });
//   };

//   render() {
//     const { title, isOfficeInitialized } = this.props;

//     if (!isOfficeInitialized) {
//       return (
//         <Progress title={title} logo="assets/logo-filled.png" message="Please sideload your addin to see app body." />
//       );
//     }

//     return (
//       <div className="ms-welcome">
//         <Header logo="assets/logo-filled.png" title={this.props.title} message="Welcome" />
//         &nbsp;
//         <a onClick={this.formatCode} href="#">
//           <i className={`ms-Icon ms-Icon--DocumentApproval`} title="Format"></i>
//           &nbsp;Format
//         </a>
//         &nbsp;&nbsp;
//         <a onClick={this.debug} href="#">
//           <i className={`ms-Icon ms-Icon--AirplaneSolid`} title="Run"></i>
//           &nbsp; Run
//         </a>
//         &nbsp;&nbsp;
//         <a onClick={this.stop} href="#">
//           <i className={`ms-Icon ms-Icon--StopSolid`} title="Stop"></i>
//           &nbsp; Stop
//         </a>
//         &nbsp;&nbsp;
//         <a onClick={this.step} href="#">
//           <i className={`ms-Icon ms-Icon--Next`} title="Step Over"></i>
//           &nbsp; Step
//         </a>
//         &nbsp;&nbsp;
//         <a onClick={this.breakpoint} href="#">
//           <i className={`ms-Icon ms-Icon--DRM`} title="Set Breakpoint"></i>
//           &nbsp; Break
//         </a>
//         <br />
//         <br />
//         <div>Status: {this.state.stateInfo} </div>
//         <br />
//         <div>Bot Messages:</div>
//         <textarea title="Bot Messages" value={this.state.conversationText} readOnly={true}></textarea>
//         <br />
//         <textarea
//           title="Message"
//           readOnly={false}
//           onKeyDown={this.sendMessage}
//         ></textarea>
//         <div>Variables:</div>
//         <div>{this.state.scope} </div>
//         <HeroList message="Discover what General Bots can do for you today!!" items={this.state.listItems}></HeroList>
//       </div>
//     );
//   }
// }
```

```js ./src/components/NavBar.js
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

import React, {PropTypes as T} from "react"


const navBar = ({ onChange, onSearch }) => (
    <div className="NavBar">
      <div className="logo">
      </div>
      
  </div>
);
export default navBar```

```js ./src/components/SEO.js
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

export default SEO```

```js ./src/components/ChatPane.js
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
import { Chat } from "botframework-webchat";

class ChatPane extends React.Component {

  render() {
    return (
      <Chat        
        ref={(chat) => { this.chat = chat; }}
        botConnection={this.props.botConnection}
        user={{ id: "webUser@gb", name: "You" }}
        bot={{ id: "bot@gb", name: "Bot" }}
      />
    );
  }
}

export default ChatPane;
```

```js ./src/index.js
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
import ReactDOM from "react-dom";
import GBUIApp from "./GBUIApp";


ReactDOM.render(
  <GBUIApp head={document.getElementsByTagName("head")[0]} />,
  document.getElementById("root")
);

```

```js ./src/GBUIApp.js
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

import React from 'react';
import GBMarkdownPlayer from './players/GBMarkdownPlayer.js';
import GBImagePlayer from './players/GBImagePlayer.js';
import GBVideoPlayer from './players/GBVideoPlayer.js';
import GBUrlPlayer from './players/GBUrlPlayer.js';
import GBMultiUrlPlayer from './players/GBMultiUrlPlayer.js';
import GBLoginPlayer from './players/GBLoginPlayer.js';
import GBBulletPlayer from './players/GBBulletPlayer.js';
import SidebarMenu from './components/SidebarMenu.js';
import SEO from './components/SEO.js';
import GBCss from './components/GBCss.js';
import { DirectLine } from 'botframework-directlinejs';
import { ConnectionStatus } from 'botframework-directlinejs';
import ReactWebChat from 'botframework-webchat';
import { UserAgentApplication } from 'msal';
import StaticContent from '@midudev/react-static-content';

class GBUIApp extends React.Component {
  constructor() {
    super();

    this.state = {
      line: null,
      token: null,
      instanceClient: null
    };
    window.user = this.getUser();
  }

  generateRandomId(length) {
    const characters = 'ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789';
    let result = '';
    const array = new Uint32Array(length);
    window.crypto.getRandomValues(array);
    for (let i = 0; i < length; i++) {
      result += characters.charAt(array[i] % characters.length);
    }
    return result;
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
        });
    }, 400);
  }

  send(command) {
    window.line.postActivity({
      type: 'event',
      name: command,
      locale: 'en-us',
      textFormat: 'plain',
      timestamp: new Date().toISOString(),
      from: this.getUser()
    });
  }

  getUser() {
    return { id: window.userId, name: 'You' };
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
          this.setupBotConnection(result);
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
      function (errorDesc, token, error) {
        if (error) {
          _this_.sendToken(error);
        }
      }
    );
    window.userAgentApplication = userAgentApplication;

    if (
      !userAgentApplication.isCallback(window.location.hash) &&
      window.parent === window &&
      !window.opener &&
      userAgentApplication.getUser
    ) {
      var user = userAgentApplication.getUser();
      if (user) {
        userAgentApplication.acquireTokenSilent(graphScopes).then(
          function (accessToken) {
            _this_.sendToken(accessToken);
          },
          function (error) {
            _this_.sendToken(error);
          }
        );
      }
    }
  }

  setupBotConnection(instanceClient) {
    let _this_ = this;
    window['botchatDebug'] = true;
    window.userId = this.generateRandomId(16);

    _this_.setState({ token: instanceClient.webchatToken });

    const line = instanceClient.webchatToken
      ? new DirectLine({
        userId:window.userId, 
        userIdOnStartConversation: window.userId, 
        token: instanceClient.webchatToken
        })
      : new DirectLine({
          domain: instanceClient.domain,
          userId:window.userId, 
          userIdOnStartConversation: window.userId, 
          secret: null,
          token: null,
          webSocket: false // defaults to true
        });
      line.setUserId(window.userId);    

    _this_.setState({ line: line });

    line.connectionStatus$.subscribe(connectionStatus => {
      if (connectionStatus === ConnectionStatus.Online) {
        _this_.setState({ instanceClient: instanceClient });
        window['botConnection'] = line;
      }
    });

    window.line = line;

    line.activity$
      .filter(activity => activity.type === 'event' && activity.name === 'loadInstance')
      .subscribe(() => {
        _this_.send('startGB');
        _this_.authenticate();
      });

    line.activity$
      .filter(activity => activity.type === 'event' && activity.name === 'stop')
      .subscribe(() => {
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

  webSpeechPonyfillFactory = 0;
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
        case 'url':
          playerComponent = (
            <GBUrlPlayer
              app={this}
              ref={player => {
                this.player = player;
              }}
            />
          );
          break;
        case 'multiurl':
          playerComponent = (
            <GBMultiUrlPlayer
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
          playerComponent = <div>GBERROR: Unknow player type specified on message from server.</div>;
          break;
      }
    }

    let chat = <div />;
    let gbCss = <div />;
    let seo = <div />;
    let sideBar = <div />;
    
    if (this.state.line) {
      
      chat = (
        <ReactWebChat
          ref={chat => {
            this.chat = chat;
          }}
          userID= {window.userId}
          locale={'en-us'}
          directLine={this.state.line}
        />
      );

      if (this.state.instanceClient) {
        let color1 = this.state.instanceClient.color1;
        gbCss = <GBCss instance={this.state.instanceClient} />;
        seo = <SEO instance={this.state.instanceClient} />;


        document.body.style.setProperty('background-color', this.state.instanceClient.color2, 'important');


        sideBar = (
          <div
            className="sidebar"
            ref={node => {
              if (node) {
                node.style.setProperty('background-color', this.state.instanceClient.color1, 'important');
              }
            }}
          >
            <SidebarMenu chat={this.chat} instance={this.state.instanceClient} />
          </div>
        );
      }
    }
    return (
      <StaticContent>
        {seo}
        <div>
          {gbCss}
          {sideBar}
          <div className="player">{playerComponent}</div>
          <div className="webchat">{chat}</div>
        </div>
      </StaticContent>
    );
  }
}

export default GBUIApp;
```

