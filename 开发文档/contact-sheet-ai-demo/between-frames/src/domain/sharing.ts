export interface SharingStatus {
  project_id: string;
  authority: "local" | "local-promoting" | "shared";
  state?: string;
  remote?: {
    webUrl?: string;
    projectId?: string;
    binding?: {
      baseUrl: string;
      webUrl: string;
      userId: string;
      workspaceId: string;
    };
  } | null;
}
export interface ShareRequest {
  projectId: string;
  expectedRevision: number;
  baseUrl: string;
  webUrl: string;
  action: "resume" | "abort";
  previewUploadAuthorized: boolean;
}
export interface SharingRepository {
  auth?: import('./cloud-auth').CloudAuthRepository;
  status(projectId: string): Promise<SharingStatus>;
  share(request: ShareRequest): Promise<SharingStatus>;
}
