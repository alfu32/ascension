-- unit.bazooka_team@modern: Bazooka team (mesh.bazooka_team_modern)
-- One composed model; loaded after seed.sql by `rom build-rules` and the game.
-- Edit here or in Meshcraft: docs/models-in-meshcraft.md.
INSERT INTO def(kind,key,name) VALUES ('mesh','mesh.bazooka_team_modern','Bazooka team');
INSERT INTO mesh(id,generator,params,baked) SELECT id,'composed','{}',1 FROM def WHERE key='mesh.bazooka_team_modern';
WITH v(name,shape,mat,team,attach,params) AS (VALUES
  ('gunner','figure','mat.cloth',1,'','{"x":-0.24,"attack":5,"hold_right":10,"legs":"#4d4c3a"}'),
  ('gunner_helmet','dome','mat.olive_drab',0,'gunner.head','{"y":-0.13,"w":0.32,"h":0.17,"segments":6}'),
  ('tube','cylinder','mat.olive_drab',0,'gunner.right_hand','{"y":-0.5,"w":0.12,"h":1.12,"segments":7}'),
  ('bell','cone','mat.olive_drab',0,'tube','{"y":0,"w":0.16,"h":0.14,"segments":7,"pitch":180,"far":0}'),
  ('grip','box','mat.timber',0,'tube','{"y":0.42,"z":0.06,"w":0.04,"d":0.1,"h":0.18,"far":0}'),
  ('spark','cone','mat.flash',0,'tube','{"y":1.12,"w":0.12,"h":0.12,"segments":3,"flash":1,"far":0}'),
  ('flash','cone','mat.flash',0,'spark','{"y":0.32,"w":0.36,"h":0.32,"segments":6,"pitch":180,"far":0}'),
  ('missile','cylinder','mat.white_metal',0,'spark','{"y":0.85,"w":0.07,"h":0.3,"segments":5,"far":0}'),
  ('trail','cone','mat.canvas',0,'missile','{"y":-0.73,"w":0.2,"h":0.75,"segments":5,"far":0}'),
  ('blast','cone','mat.flash',0,'tube','{"y":0,"w":0.12,"h":0.12,"segments":3,"pitch":180,"flash":1,"far":0}'),
  ('blast_cloud','cone','mat.canvas',0,'blast','{"y":0.4,"w":0.4,"h":0.4,"segments":6,"pitch":180,"far":0}'),
  ('loader','figure','mat.cloth',1,'','{"x":0.34,"z":-0.25,"yaw":-20,"attack":7,"hold_right":40,"hold_left":40,"legs":"#4d4c3a"}'),
  ('loader_helmet','dome','mat.olive_drab',0,'loader.head','{"y":-0.13,"w":0.32,"h":0.17,"segments":6}'),
  ('rocket','cylinder','mat.olive_drab',0,'loader.right_hand','{"x":0.1,"y":0,"w":0.07,"h":0.36,"segments":5,"roll":90,"far":0}'),
  ('rocket_bag','box','mat.khaki',0,'loader.back','{"y":-0.2,"z":-0.04,"w":0.22,"d":0.12,"h":0.3}'))
INSERT INTO mesh_part(mesh_id,name,shape,material_id,team,attach,params)
 SELECT (SELECT id FROM def WHERE key='mesh.bazooka_team_modern'), v.name, v.shape, (SELECT id FROM def WHERE key=v.mat), v.team, v.attach, v.params FROM v;
INSERT INTO entity_visual(entity_type_id,faction_id,age_id,mesh_id,material_id,scale_pct) VALUES
 ((SELECT id FROM def WHERE key='unit.bazooka_team'), NULL, (SELECT id FROM def WHERE key='age.modern'), (SELECT id FROM def WHERE key='mesh.bazooka_team_modern'), (SELECT id FROM def WHERE key='mat.cloth'), 100);
