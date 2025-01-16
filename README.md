<p align="center">
  <a href="https://picaos.com">
    <picture>
      <source media="(prefers-color-scheme: dark)" srcset="./resources/images/logo-dark.svg">
      <source media="(prefers-color-scheme: light)" srcset="./resources/images/logo-light.svg">
      <img alt="Pica Logo" src="./resources/images/logo-light.svg" height="100px">
    </picture>
  </a>
</p>

<p align="center"><b>The Complete Agentic Infrastructure</b></p>

<p align="center">
  <b>
    <a href="https://www.picaos.com/">Website</a>
    ·
    <a href="https://docs.picaos.com">Documentation</a>
    ·
    <a href="https://www.picaos.com/community">Community Hub</a>
    ·
    <a href="https://www.picaos.com/community/changelog">Changelog</a>
    ·
    <a href="https://twitter.com/picahq">Twitter</a>
  </b>
</p>

---

Stop wrestling with UI libraries, hacking together data models, and figuring out authentication. Start shipping native integrations that move your business forward.

Access realtime data from any integration using a single API
Forget the pain of having to manually parse through, transform and maintain hundreds of data models. Now integrating bi-directional flows for a new integration is as simple as calling a single API.

# Get started

To get the most out of this guide, you'll need:

1. A [Pica account](https://app.picaos.com)
2. Your Pica [API Key](https://docs.picaos.com/docs/glossary#api-key)

## Step 1: Backend - Create secure tokens

First, we'll add an endpoint to our backend that'll let us generate secure tokens for our frontend component.

### Install the SDK

To make this easy, Pica offers native SDKs in several popular programming languages. This guide will use the popular AuthKit SDK for Node.js.

```shell npm
npm install @picahq/authkit-node
```

### Set secrets

To make calls to Pica, provide your API key. Store these values as managed secrets and pass them to the SDKs either as environment variables or directly in your app's configuration depending on your preferences.

```shell
PICA_SANDBOX_API_KEY='sk_test_example_123456789'
PICA_PRODUCTION_API_KEY='sk_live_example_123456789'
```

### Create a token endpoint

Next, we'll need to add the token endpoint which will exchange the authorization token (valid for 10 minutes) for an authenticated Connected Account.

```javascript
import { AuthKitToken } from "@picahq/authkit-node";

app.post("/authkit-token", async (request, response) => {
  const authKitToken = new AuthKitToken(process.env.PICA_SANDBOX_API_KEY);

  // Specifying how the token will be constructed
  const token = await authKitToken.create({
    identity: orgId // a meaningful identifier (i.e., userId, teamId or organizationId)
    identityType: "organization" // this can either be "user", "team" or "organization"
  });

  response.send(token);

});
```

## Step 2: Frontend - Make AuthKit appear

Next, we'll add the AuthKit component to your frontend application.

### Install the SDK

In the same fashion, Pica offers native frontend SDKs in several popular frameworks. Compatible with React, Next.js, Vue, Svelte and more.

```shell npm
npm install @picahq/authkit
```

### Use the AuthKit Component

Next, we need to add the AuthKit component and replace the token URL with the URL of the token endpoint URL you created in Step 1 of this guide.

```javascript
import { useAuthKit } from "@picahq/authkit";

const { open } = useAuthKit({
  token: {
    url: "https://api.your-company-name.com/authkit-token",
    headers: {},
  },
  onSuccess: (connection) => {},
  onError: (error) => {},
  onClose: () => {},
});
```

### Launch the AuthKit flow

With your client and server setup complete, you can now test the authentication flow by calling open().

```javascript
<button onClick={open}>Add new integration</button>
```

This will open the AuthKit modal so your user can:

- Select an integration to connect
- Be guided through the authentication flow
- Authorize data access

Once the flow is completed, AuthKit will return a Connection object to your onSuccess callback. Each connection object contains metadata about the connected account and can be used to make API requests.

View the full guide [here](https://docs.picaos.com).

# Running Pica locally

## Prerequisites

* [Docker](https://docs.docker.com/engine/) and [Docker Compose](https://docs.docker.com/compose/)

## Setup

1. Copy `.env-example` to `.env`. Review and update the environment variables.

2. Run the containers

    ```shell
    docker compose up -d
    ```
3. Run migrations and load seed data

    ```shell
    docker compose -f docker-compose.data.yml run --rm migrate-before
    docker compose -f docker-compose.data.yml run --rm migrate-after
    docker compose -f docker-compose.data.yml run --rm seed-data
    ```

**Note:** If you want to run the latest version of the docker image, you can use the latest git commit hash as the tag. For example, `picahq/pica:<commit-hash>`.

## Other actions

Connecting to a MongoDB shell

```shell
source .env
docker compose exec mongo mongosh -u pica -p $MONGO_PASSWORD --authenticationDatabase=admin events-service
```


# License

Pica is released under the [**GPL-3.0 license**](LICENSE).
