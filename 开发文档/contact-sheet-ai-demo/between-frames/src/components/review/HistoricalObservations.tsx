import type { HistoricalPhotoObservation } from '../../domain/revisionContext';
import { zh } from '../../app/copy';

export function HistoricalObservations({ observations, total }: { observations: HistoricalPhotoObservation[]; total: number }) {
  return <section className="revision-observations" aria-label={zh.editorialRevisionObservations}>
    <h5>{zh.editorialRevisionObservations} · {total}</h5><p className="muted">{zh.editorialRevisionObservationHint}</p>
    {observations.map(photo => <article className="revision-observation" key={`${photo.taskId}:${photo.callId}:${photo.photoId}`}>
      <strong>{photo.photoName}</strong>
      {photo.currentHashMatches && photo.thumbnailPath && <img className="revision-evidence-thumbnail" src={photo.thumbnailPath} alt={photo.photoName} loading="lazy" />}
      {!photo.currentHashMatches && <p role="status">{zh.editorialRevisionPhotoChanged}</p>}
      <ul>{photo.visibleFacts.map((fact,index) => <li key={index}>{fact}</li>)}</ul><p>{photo.interpretation}</p><p className="muted">{photo.uncertainty}</p>
    </article>)}
  </section>;
}
