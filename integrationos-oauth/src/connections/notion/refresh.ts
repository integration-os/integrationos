import { DataObject, OAuthResponse } from '../../lib/types';

export const refresh = async ({ body }: DataObject): Promise<OAuthResponse> => {
    try {
        const {
            OAUTH_ACCESS_TOKEN: accessToken,
            OAUTH_REFRESH_TOKEN: refreshToken,
            OAUTH_METADATA: { meta },
        } = body;

        return {
            accessToken,
            refreshToken,
            expiresIn: 2147483647,
            tokenType: 'Bearer',
            meta,
        };
    } catch (error) {
        throw new Error(`Error fetching refresh token for Notion: ${error}`);
    }
};
