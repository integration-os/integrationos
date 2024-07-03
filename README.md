<p align="center">
📖 Just launched <b><a href="https://www.integrationos.com/">IntegrationOS</a></b>
  <br/>
 </p>

<p align="center">
  <a href="https://integrationos.com">
    <img src="https://assets-global.website-files.com/5f6b97302bb70b93e591d51f/657a3a1aec47c8ec20b396fe_IntegrationOS%20main%20logo-p-500.png" height="64px">
  </a>
</p>

<p align="center"><b>Ship integrations, remarkably fast.</b></p>

<p align="center">
  <b>
    <a href="https://www.integrationos.com/">Website</a>
    ·
    <a href="https://docs.integrationos.com/docs/quickstart">Documentation</a>
    ·
    <a href="https://www.integrationos.com/changelog">Changelog</a>
    ·
    <a href="https://www.integrationos.com/blog">Blog</a>
    ·
    <a href="https://join.slack.com/t/integrationos-hq/shared_invite/zt-2dm9254tc-Eza~78acJllbP7ZFKuVYjw">Slack</a>
    ·
    <a href="https://twitter.com/integrationos">Twitter</a>
  </b>
</p>

---

Stop wrestling with UI libraries, hacking together data models, and figuring out authentication. Start shipping native integrations that move your business forward.

Access realtime data from any integration using a single API
Forget the pain of having to manually parse through, transform and maintain hundreds of data models. Now integrating bi-directional flows for a new integration is as simple as calling a single API.

# Get started

To get the most out of this guide, you'll need:

1. An [IntegrationOS account](https://app.integrationos.com)
2. Your  IntegrationOS [API Key](https://docs.integrationos.com/docs/glossary#api-key)

## Step 1: Backend - Create secure tokens

First, we'll add an endpoint to our backend that'll let us generate secure tokens for our frontend component.

### Install the SDK

To make this easy, IntegrationOS offers native SDKs in several popular programming languages. This guide will use the popular AuthKit SDK for Node.js.

```shell npm
npm install @integrationos/authkit-node
```

### Set secrets

To make calls to IntegrationOS, provide your API key. Store these values as managed secrets and pass them to the SDKs either as environment variables or directly in your app's configuration depending on your preferences.

```shell
INTEGRATIONOS_SANDBOX_API_KEY='sk_test_example_123456789'
INTEGRATIONOS_PRODUCTION_API_KEY='sk_live_example_123456789'
```

### Create a token endpoint

Next, we'll need to add the token endpoint which will exchange the authorization token (valid for 10 minutes) for an authenticated Connected Account.

```javascript
import { AuthKitToken } from "@integrationos/authkit-node";

app.post("/authkit-token", async (request, response) => {
  const authKitToken = new AuthKitToken(process.env.INTEGRATIONOS_SANDBOX_API_KEY);

// Specifying how the token will be constructed
  const token = await authKitToken.create({
    group: "org_123", // a meaningful identifier (i.e., organizationId)
    label: "Acme" // a human-friendly label (i.e., organizationName)
  });

  response.send(token);

});
```

## Step 2: Frontend - Make AuthKit appear

Next, we'll add the AuthKit component to your frontend application.

### Install the SDK

In the same fashion, IntegrationOS offers native frontend SDKs in several popular frameworks. Compatible with React, Next.js, Vue, Svelte and more.

```shell npm
npm install @integrationos/authkit
```

### Use the AuthKit Component

Next, we need to add the AuthKit component and replace the token URL with the URL of the token endpoint URL you created in Step 1 of this guide.

```javascript
import { useAuthKit } from "@integrationos/authkit";

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

View the full guide [here](https://docs.integrationos.com/docs/quickstart).

# Running IntegrationOS locally

## Prerequisites

* [Docker](https://docs.docker.com/engine/) and [Docker Compose](https://docs.docker.com/compose/)
* A [Google Cloud KMS](https://cloud.google.com/kms/docs) key ring
* [`gcloud`](https://cloud.google.com/sdk/gcloud) installed, logged into an account that has `roles/cloudkms.cryptoKeyEncrypterDecrypter` access, and configured with [Application Default Credentials](https://cloud.google.com/docs/authentication/provide-credentials-adc)

## Setup

1. Copy `.env-example` to `.env`. Review and update the environment variables.

2. Run the containers

    ```shell
    docker-compose up -d
    ```
3. Run migrations and load seed data

    ```shell
    docker-compose -f docker-compose.data.yml run --rm migrate-before
    docker-compose -f docker-compose.data.yml run --rm migrate-after
    docker-compose -f docker-compose.data.yml run --rm seed-data
    ```

**Note:** If you want to run the latest version of the docker image, you can use the latest git commit hash as the tag. For example, `integrationos/integrationos:<commit-hash>`.

## Other actions

Connecting to a MongoDB shell

```shell
source .env
docker-compose exec mongo mongosh -u integrationos -p $MONGO_PASSWORD --authenticationDatabase=admin events-service
```


# License

IntegrationOS is released under the [**GPL-3.0 license**](LICENSE).
