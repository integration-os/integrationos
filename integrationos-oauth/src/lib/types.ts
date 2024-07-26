export interface DataObject {
  [key: string]: any;
}

export interface OAuthResponse {
  accessToken: string;
  refreshToken: string;
  // expiresIn will be in seconds
  expiresIn: number;
  tokenType: string;
  meta: DataObject;
}
