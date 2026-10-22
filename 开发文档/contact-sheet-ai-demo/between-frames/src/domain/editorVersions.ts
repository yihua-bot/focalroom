export interface EditorVersion {
  id: string; photoName: string; provider: 'lightroom_classic' | 'capture_one';
  previewSrc: string; createdAt: string;
}
export interface EditorVersionsRepository {
  list(projectId: string): Promise<EditorVersion[]>;
  import(projectId: string): Promise<number | null>;
}
